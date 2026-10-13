//! Assisted installation: immutable reviewed plan -> explicit confirmation -> worker -> verification.
use crate::{
    Host, OperatingSystem,
    engine::{ProbeState, probe, resolve},
};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    Pacman,
    Apt,
    Dnf,
    Brew,
    Winget,
}
#[derive(Debug, Clone)]
pub struct SetupHost {
    pub manager: Option<(Manager, PathBuf)>,
    pub distribution: String,
    pub sudo: Option<PathBuf>,
}
impl SetupHost {
    pub fn detect(host: &Host) -> Self {
        let distro = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let distribution = distro
            .lines()
            .find_map(|l| l.strip_prefix("PRETTY_NAME="))
            .map(|s| s.trim_matches('"').to_owned())
            .unwrap_or_else(|| format!("{:?}", host.os));
        let manager = match host.os {
            OperatingSystem::Windows => resolve("winget", &host.path)
                .into_iter()
                .next()
                .map(|p| (Manager::Winget, p)),
            OperatingSystem::MacOS => ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
                .into_iter()
                .map(PathBuf::from)
                .find(|p| p.is_file())
                .map(|p| (Manager::Brew, p)),
            OperatingSystem::Linux => {
                let families = distro
                    .lines()
                    .filter_map(|l| l.strip_prefix("ID=").or_else(|| l.strip_prefix("ID_LIKE=")))
                    .collect::<Vec<_>>()
                    .join(" ");
                let choices = if families.contains("arch") {
                    vec![(Manager::Pacman, "/usr/bin/pacman")]
                } else if families.contains("debian") || families.contains("ubuntu") {
                    vec![(Manager::Apt, "/usr/bin/apt-get")]
                } else if families.contains("fedora") {
                    vec![(Manager::Dnf, "/usr/bin/dnf")]
                } else {
                    Vec::new()
                };
                choices
                    .into_iter()
                    .find_map(|(m, p)| Path::new(p).is_file().then(|| (m, PathBuf::from(p))))
            }
            _ => None,
        };
        let sudo = PathBuf::from("/usr/bin/sudo");
        Self {
            manager,
            distribution,
            sudo: sudo.is_file().then_some(sudo),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Plan {
    tool: String,
    program: PathBuf,
    arguments: Vec<String>,
    pub method: String,
    pub explanation: String,
}
pub struct ConfirmedPlan(Plan);
impl Plan {
    pub fn for_tool(tool: &str, setup: &SetupHost) -> Option<Self> {
        let (manager, path) = setup.manager.as_ref()?;
        let package = package(*manager, tool)?;
        let (program, arguments) = match manager {
            Manager::Pacman | Manager::Apt | Manager::Dnf => {
                let mut args = vec!["-n".into(), "--".into(), path.display().to_string()];
                args.extend(
                    match manager {
                        Manager::Pacman => vec!["-S", "--needed", "--noconfirm", package],
                        Manager::Apt => vec!["install", "-y", package],
                        _ => vec!["install", "-y", package],
                    }
                    .into_iter()
                    .map(String::from),
                );
                (setup.sudo.clone()?, args)
            }
            Manager::Brew => (path.clone(), vec!["install".into(), package.into()]),
            Manager::Winget => (
                path.clone(),
                vec![
                    "install".into(),
                    "--id".into(),
                    package.into(),
                    "--exact".into(),
                    "--source".into(),
                    "winget".into(),
                    "--disable-interactivity".into(),
                ],
            ),
        };
        Some(Self {
            tool: tool.into(),
            program,
            arguments,
            method: format!("{manager:?} · {}", setup.distribution),
            explanation: if matches!(manager, Manager::Pacman | Manager::Apt | Manager::Dnf) {
                "Installs a system package and its dependencies. sudo -n never requests or stores your password. If administrator authorization is unavailable, execution fails; you may run the reviewed command yourself in an external terminal. No repository refresh, system upgrade or AUR helper is run.".into()
            } else {
                "Installs the selected package and dependencies using the detected manager. Network access may be required. Package agreements or administrator requirements can prevent noninteractive installation; no agreements are accepted on your behalf.".into()
            },
        })
    }
    pub fn command(&self) -> String {
        let command = std::iter::once(self.program.display().to_string())
            .chain(self.arguments.iter().cloned())
            .map(|s| {
                let escaped = if cfg!(windows) {
                    s.replace('\'', "''")
                } else {
                    s.replace('\'', "'\\''")
                };
                format!("'{escaped}'")
            })
            .collect::<Vec<_>>()
            .join(" ");
        if cfg!(windows) {
            format!("& {command}")
        } else {
            command
        }
    }
    pub fn tool(&self) -> &str {
        &self.tool
    }
    /// The application must call this only from its explicit confirmation action.
    pub fn confirm(self) -> ConfirmedPlan {
        ConfirmedPlan(self)
    }
}
fn package(manager: Manager, tool: &str) -> Option<&'static str> {
    Some(match manager {
        Manager::Pacman => match tool {
            "rustc" | "cargo" | "rustfmt" | "cargo-clippy" => "rust",
            "rust-analyzer" => "rust-analyzer",
            "gcc" | "g++" => "gcc",
            "clang" | "clang++" | "clang-format" | "clangd" => "clang",
            "cmake" => "cmake",
            "make" => "make",
            "ninja" => "ninja",
            "gdb" => "gdb",
            "lldb" => "lldb",
            "python" | "python3" => "python",
            "pip" | "pip3" => "python-pip",
            "pipx" => "python-pipx",
            "uv" => "uv",
            "poetry" => "python-poetry",
            "java" | "javac" => "jdk-openjdk",
            "mvn" => "maven",
            "gradle" => "gradle",
            "node" => "nodejs",
            "npm" => "npm",
            "pnpm" => "pnpm",
            "yarn" => "yarn",
            "go" | "gofmt" => "go",
            _ => return None,
        },
        Manager::Apt => match tool {
            "rustc" => "rustc",
            "cargo" => "cargo",
            "rustfmt" => "rustfmt",
            "gcc" => "gcc",
            "g++" => "g++",
            "clang" | "clang++" => "clang",
            "clang-format" => "clang-format",
            "clangd" => "clangd",
            "cmake" => "cmake",
            "make" => "make",
            "ninja" => "ninja-build",
            "gdb" => "gdb",
            "lldb" => "lldb",
            "python3" => "python3",
            "pip" | "pip3" => "python3-pip",
            "pipx" => "pipx",
            "java" | "javac" => "default-jdk",
            "mvn" => "maven",
            "gradle" => "gradle",
            "node" => "nodejs",
            "npm" => "npm",
            "go" | "gofmt" => "golang-go",
            _ => return None,
        },
        Manager::Dnf => match tool {
            "rustc" => "rust",
            "cargo" => "cargo",
            "gcc" => "gcc",
            "g++" => "gcc-c++",
            "clang" | "clang++" => "clang",
            "cmake" => "cmake",
            "make" => "make",
            "ninja" => "ninja-build",
            "gdb" => "gdb",
            "lldb" => "lldb",
            "python3" => "python3",
            "pip" | "pip3" => "python3-pip",
            "node" | "npm" => "nodejs",
            "go" | "gofmt" => "golang",
            _ => return None,
        },
        Manager::Brew => match tool {
            "rustc" | "cargo" | "rustfmt" => "rust",
            "rust-analyzer" => "rust-analyzer",
            "clang" | "clang++" | "clang-format" | "clangd" | "lldb" => "llvm",
            "gcc" | "g++" => "gcc",
            "cmake" => "cmake",
            "make" => "make",
            "ninja" => "ninja",
            "python3" | "pip3" => "python",
            "pipx" => "pipx",
            "uv" => "uv",
            "poetry" => "poetry",
            "java" | "javac" => "openjdk",
            "mvn" => "maven",
            "gradle" => "gradle",
            "node" | "npm" => "node",
            "pnpm" => "pnpm",
            "yarn" => "yarn",
            "go" | "gofmt" => "go",
            _ => return None,
        },
        Manager::Winget => match tool {
            "rustc" | "cargo" => "Rustlang.Rustup",
            "node" | "npm" => "OpenJS.NodeJS.LTS",
            "cmake" => "Kitware.CMake",
            "clang" | "clang++" => "LLVM.LLVM",
            "go" | "gofmt" => "GoLang.Go",
            _ => return None,
        },
    })
}
#[derive(Debug)]
pub enum InstallEvent {
    Output(String),
    Finished(Result<ProbeState, String>),
}
pub struct Installation {
    pub events: Receiver<InstallEvent>,
}
impl ConfirmedPlan {
    pub fn execute(self) -> Installation {
        let (tx, events) = mpsc::sync_channel(128);
        thread::spawn(move || {
            let plan = self.0;
            let result = (|| {
                let mut child = Command::new(&plan.program)
                    .args(&plan.arguments)
                    .current_dir(std::env::temp_dir())
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .map_err(|e| e.to_string())?;
                let mut readers = Vec::new();
                for stream in [
                    child
                        .stdout
                        .take()
                        .map(|s| Box::new(s) as Box<dyn Read + Send>),
                    child
                        .stderr
                        .take()
                        .map(|s| Box::new(s) as Box<dyn Read + Send>),
                ]
                .into_iter()
                .flatten()
                {
                    let tx = tx.clone();
                    readers.push(thread::spawn(move || {
                        let mut stream = stream;
                        let mut buffer = [0u8; 2048];
                        loop {
                            match stream.read(&mut buffer) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => {
                                    let text = String::from_utf8_lossy(&buffer[..n])
                                        .chars()
                                        .filter(|c| !c.is_control() || *c == '\n')
                                        .collect();
                                    let _ = tx.try_send(InstallEvent::Output(text));
                                }
                            }
                        }
                    }));
                }
                let status = child.wait().map_err(|e| e.to_string())?;
                for reader in readers {
                    let _ = reader.join();
                }
                if !status.success() {
                    return Err(format!(
                        "Package manager exited with {status}. No successful installation is claimed."
                    ));
                }
                let host = Host::detect();
                let mut directories = host.path.clone();
                directories.extend([
                    PathBuf::from("/usr/bin"),
                    PathBuf::from("/usr/local/bin"),
                    PathBuf::from("/opt/homebrew/bin"),
                ]);
                let spec = crate::providers::registry()
                    .into_iter()
                    .flat_map(|p| {
                        directories.extend(p.extra_directories(&host));
                        p.specifications()
                    })
                    .find(|s| s.name == plan.tool);
                let path=resolve(&plan.tool,&directories).into_iter().next().ok_or("Manager completed, but the executable is not visible. Restart or update PATH and scan again.")?;
                Ok(probe(
                    &path,
                    spec.as_ref().map_or(&["--version"], |s| s.arguments),
                    Duration::from_secs(3),
                ))
            })();
            let _ = tx.send(InstallEvent::Finished(result));
        });
        Installation { events }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plan_is_reviewable_and_allowlisted() {
        let host = SetupHost {
            manager: Some((Manager::Pacman, "/usr/bin/pacman".into())),
            distribution: "Arch".into(),
            sudo: Some("/usr/bin/sudo".into()),
        };
        let plan = Plan::for_tool("rustc", &host).expect("supported");
        assert!(plan.command().contains("'-n'"));
        assert!(plan.command().contains("'rust'"));
        assert!(Plan::for_tool("rustc; touch /tmp/pwned", &host).is_none());
        assert!(Plan::for_tool("bun", &host).is_none());
    }
    #[test]
    fn no_linux_elevation_without_sudo() {
        let h = SetupHost {
            manager: Some((Manager::Apt, "/usr/bin/apt-get".into())),
            distribution: "Debian".into(),
            sudo: None,
        };
        assert!(Plan::for_tool("cargo", &h).is_none());
    }
    #[cfg(unix)]
    #[test]
    fn successful_manager_without_tool_is_not_verified() {
        let plan = ConfirmedPlan(Plan {
            tool: "buture-missing-tool-fixture".into(),
            program: "/usr/bin/true".into(),
            arguments: Vec::new(),
            method: "test".into(),
            explanation: "test".into(),
        });
        let installation = plan.execute();
        match installation
            .events
            .recv_timeout(Duration::from_secs(5))
            .expect("worker event")
        {
            InstallEvent::Finished(Err(message)) => assert!(message.contains("not visible")),
            event => panic!("Unexpected event: {event:?}"),
        }
    }
}
