//! Diagnostics describe evidence, scope and actionable causes; never invent tool or service states.
use buture_toolchains::{
    Host, OperatingSystem,
    engine::{ProbeState, Role, Toolchain, probe, resolve},
};
use std::{path::PathBuf, sync::mpsc, time::Duration};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ready,
    NeedsAttention,
    Blocked,
}
impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::NeedsAttention => "Needs Attention",
            Self::Blocked => "Blocked",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Solution {
    Manual(String),
    InstallTool(String),
}
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub category: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub cause: String,
    pub solution: Solution,
}
#[derive(Debug, Clone)]
pub struct Report {
    pub status: Status,
    pub diagnostics: Vec<Diagnostic>,
    pub scope: String,
}
#[derive(Debug, Clone)]
pub struct Facts {
    pub git: ProbeState,
    pub docker: ProbeState,
    pub docker_daemon: Option<ProbeState>,
    pub listening_ports: Option<usize>,
    pub path_directories: usize,
    pub shell_known: bool,
    pub home_known: bool,
    pub project_manifests: Vec<String>,
}
impl Facts {
    pub fn observe(host: &Host, root: Option<&std::path::Path>) -> Self {
        let version = |name: &str| {
            resolve(name, &host.path)
                .into_iter()
                .next()
                .map_or(ProbeState::Missing, |p| {
                    probe(&p, &["--version"], Duration::from_secs(3))
                })
        };
        let git = version("git");
        let docker = version("docker");
        let docker_daemon = resolve("docker", &host.path).into_iter().next().map(|p| {
            probe(
                &p,
                &["info", "--format", "{{.ServerVersion}}"],
                Duration::from_secs(3),
            )
        });
        let listening_ports = if host.os == OperatingSystem::Linux {
            ["/proc/net/tcp", "/proc/net/tcp6"]
                .into_iter()
                .map(std::fs::read_to_string)
                .collect::<Result<Vec<_>, _>>()
                .ok()
                .map(|tables| {
                    tables
                        .iter()
                        .flat_map(|s| s.lines().skip(1))
                        .filter(|line| line.split_whitespace().nth(3) == Some("0A"))
                        .count()
                })
        } else {
            None
        };
        let project_manifests = root
            .map(|r| {
                [
                    "Cargo.toml",
                    "package.json",
                    "pyproject.toml",
                    "requirements.txt",
                    "go.mod",
                    "pom.xml",
                    "build.gradle",
                    "CMakeLists.txt",
                    "Makefile",
                ]
                .into_iter()
                .filter(|name| r.join(name).is_file())
                .map(String::from)
                .collect()
            })
            .unwrap_or_default();
        Self {
            git,
            docker,
            docker_daemon,
            listening_ports,
            path_directories: host.path.len(),
            shell_known: host.shell.is_some(),
            home_known: host.home.is_some(),
            project_manifests,
        }
    }
}
fn diagnostic(
    category: &str,
    title: impl Into<String>,
    description: impl Into<String>,
    severity: Severity,
    cause: impl Into<String>,
    solution: Solution,
) -> Diagnostic {
    Diagnostic {
        category: category.into(),
        title: title.into(),
        description: description.into(),
        severity,
        cause: cause.into(),
        solution,
    }
}
fn manual(text: &str) -> Solution {
    Solution::Manual(text.into())
}
pub fn diagnose(reports: &[Toolchain], facts: &Facts, required: &[String]) -> Report {
    let mut diagnostics = Vec::new();
    diagnostics.push(diagnostic(
        "Environment",
        "PATH",
        format!(
            "{} absolute search directories observed",
            facts.path_directories
        ),
        if facts.path_directories == 0 {
            Severity::Error
        } else {
            Severity::Info
        },
        if facts.path_directories == 0 {
            "No absolute executable search path supplied"
        } else {
            "Host environment supplied PATH"
        },
        manual("Review your host PATH; Doctor does not change environment variables."),
    ));
    diagnostics.push(diagnostic(
        "Environment",
        "Home and shell",
        format!(
            "Home supplied: {} · shell supplied: {}",
            facts.home_known, facts.shell_known
        ),
        if facts.home_known {
            Severity::Info
        } else {
            Severity::Warning
        },
        "Host variables inspected; no private values displayed",
        manual("Set the host home/shell explicitly if they are missing."),
    ));
    for report in reports {
        let relevant = required.iter().any(|l| {
            l == &report.language
                || (report.language == "C/C++" && ["C", "C++"].contains(&l.as_str()))
                || (report.language == "Node.js" && l == "TypeScript")
        });
        if !relevant {
            continue;
        }
        let roles = if report.language == "C/C++" {
            vec![Role::Compiler]
        } else if ["Python", "Node.js"].contains(&report.language.as_str()) {
            vec![Role::Runtime]
        } else {
            vec![Role::Compiler, Role::BuildTool, Role::PackageManager]
        };
        for role in roles {
            let tools: Vec<_> = report.tools.iter().filter(|t| t.role == role).collect();
            if tools.is_empty() {
                continue;
            }
            if !tools
                .iter()
                .any(|t| matches!(t.state, ProbeState::Ready(_)))
            {
                let exists = tools.iter().any(|t| t.executable.is_some());
                diagnostics.push(diagnostic(
                    &report.language,
                    format!("{role:?} unavailable"),
                    "No verified executable for this required role",
                    if exists {
                        Severity::Warning
                    } else {
                        Severity::Error
                    },
                    if exists {
                        "Executable was detected, but its version probe failed"
                    } else {
                        "No candidate found in known locations"
                    },
                    if exists {
                        manual("Inspect tool details and repair the host configuration manually.")
                    } else {
                        Solution::InstallTool(tools[0].name.clone())
                    },
                ));
            }
        }
        let mut names = std::collections::BTreeSet::new();
        for tool in &report.tools {
            if !names.insert(&tool.name) {
                continue;
            }
            let matching: Vec<_> = report
                .tools
                .iter()
                .filter(|t| t.name == tool.name)
                .collect();
            if let Some(ready) = matching
                .iter()
                .find(|t| matches!(t.state, ProbeState::Ready(_)))
            {
                let version = match &ready.state {
                    ProbeState::Ready(v) => v.as_str(),
                    _ => "",
                };
                diagnostics.push(diagnostic(
                    &report.language,
                    &tool.name,
                    version,
                    Severity::Info,
                    "Version verified",
                    manual("No action required."),
                ));
            } else {
                let exists = matching.iter().any(|t| t.executable.is_some());
                let optional = matches!(
                    tool.role,
                    Role::Debugger | Role::Formatter | Role::LanguageServer
                );
                diagnostics.push(diagnostic(&report.language,&tool.name,if exists{"Detected · version not verified"}else{"Not found"},if optional || exists{Severity::Warning}else{Severity::Info},if exists{"Version command failed or timed out"}else{"No executable in the scanned directories"},if exists{manual("Review Toolchains details; Doctor does not restart or modify tools automatically.")}else{Solution::InstallTool(tool.name.clone())}));
            }
        }
    }
    for (category, name, state) in [
        ("Git", "Git", &facts.git),
        ("Docker", "Docker CLI", &facts.docker),
    ] {
        diagnostics.push(diagnostic(
            category,
            name,
            match state {
                ProbeState::Ready(s) => s.clone(),
                ProbeState::Missing => "Not found".into(),
                ProbeState::Failed(s) => format!("Probe failed: {s}"),
            },
            Severity::Info,
            "Optional host tool inspected",
            manual("Install or configure this optional tool if your project requires it."),
        ));
    }
    if let Some(state) = &facts.docker_daemon {
        diagnostics.push(diagnostic(
            "Services",
            "Docker daemon",
            match state {
                ProbeState::Ready(s) => format!("Daemon responded: {s}"),
                _ => "Daemon query failed; availability not verified".into(),
            },
            Severity::Info,
            "Read-only docker info query",
            manual("Inspect Docker permissions and start its service manually when needed."),
        ));
    } else {
        diagnostics.push(diagnostic(
            "Services",
            "Service discovery",
            "Docker cannot be queried; arbitrary services have not been inspected",
            Severity::Info,
            "No service requirements defined yet",
            manual("Project-specific service checks belong to later Doctor expansion."),
        ));
    }
    diagnostics.push(diagnostic(
        "Ports",
        "Listening TCP sockets",
        facts.listening_ports.map_or_else(
            || "Not inspected on this platform".into(),
            |n| format!("{n} listening sockets observed"),
        ),
        Severity::Info,
        "Linux proc tables, when available; no port conflicts inferred",
        manual("Review project port requirements before changing any process."),
    ));
    diagnostics.push(diagnostic(
        "Project",
        "Project manifests",
        if facts.project_manifests.is_empty() {
            "No recognized manifest at workspace root".into()
        } else {
            facts.project_manifests.join(", ")
        },
        Severity::Info,
        "Read-only filesystem inspection",
        manual("Open a project or inspect the detected stack in Project Overview."),
    ));
    if reports.is_empty() || required.is_empty() {
        diagnostics.push(diagnostic(
            "Environment",
            "Scan scope",
            "Select a project or development languages and scan tools",
            Severity::Warning,
            "Required toolchains have not been fully established",
            manual("Run Doctor after choosing languages or opening a recognized project."),
        ));
    }
    let status = if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        Status::Blocked
    } else if diagnostics.iter().any(|d| d.severity == Severity::Warning) {
        Status::NeedsAttention
    } else {
        Status::Ready
    };
    Report {
        status,
        diagnostics,
        scope: if required.is_empty() {
            "Host observations; no required languages selected".into()
        } else {
            format!("Required languages: {}", required.join(", "))
        },
    }
}
pub struct Scan {
    pub events: mpsc::Receiver<Report>,
}
impl Scan {
    pub fn start(
        host: Host,
        reports: Vec<Toolchain>,
        root: Option<PathBuf>,
        required: Vec<String>,
    ) -> Self {
        let (tx, events) = mpsc::channel();
        std::thread::spawn(move || {
            let facts = Facts::observe(&host, root.as_deref());
            let _ = tx.send(diagnose(&reports, &facts, &required));
        });
        Self { events }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use buture_toolchains::engine::Tool;
    fn facts() -> Facts {
        Facts {
            git: ProbeState::Missing,
            docker: ProbeState::Missing,
            docker_daemon: None,
            listening_ports: None,
            path_directories: 2,
            shell_known: true,
            home_known: true,
            project_manifests: Vec::new(),
        }
    }
    #[test]
    fn alternatives_and_optional_tools_do_not_fabricate_blockers() {
        let report = Toolchain {
            language: "C/C++".into(),
            environments: Vec::new(),
            tools: vec![
                Tool {
                    name: "gcc".into(),
                    role: Role::Compiler,
                    executable: None,
                    state: ProbeState::Missing,
                },
                Tool {
                    name: "clang".into(),
                    role: Role::Compiler,
                    executable: Some("/clang".into()),
                    state: ProbeState::Ready("clang 22".into()),
                },
            ],
        };
        assert_eq!(
            diagnose(std::slice::from_ref(&report), &facts(), &["C++".into()]).status,
            Status::Ready
        );
        assert_eq!(
            diagnose(&[report], &facts(), &[]).status,
            Status::NeedsAttention
        );
    }
    #[test]
    fn absent_required_runtime_blocks() {
        let report = Toolchain {
            language: "Python".into(),
            environments: Vec::new(),
            tools: vec![Tool {
                name: "python3".into(),
                role: Role::Runtime,
                executable: None,
                state: ProbeState::Missing,
            }],
        };
        assert_eq!(
            diagnose(&[report], &facts(), &["Python".into()]).status,
            Status::Blocked
        );
    }
}
