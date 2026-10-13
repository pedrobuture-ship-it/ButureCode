//! Manifest-based project detection. Project files are data, never executable instructions.
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Rust,
    Node,
    Python,
    Go,
    Java,
    Cpp,
    DotNet,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildSystem {
    Cargo,
    NodeScripts,
    Python,
    Go,
    Maven,
    Gradle,
    CMake,
    Make,
    DotNet,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCandidate {
    pub label: String,
    pub program: String,
    pub arguments: Vec<String>,
    pub directory: PathBuf,
    pub requires_workspace_trust: bool,
}
#[derive(Debug, Clone)]
pub struct ProjectMetadata {
    pub name: String,
    pub kind: ProjectKind,
    pub root: PathBuf,
    pub manifest: PathBuf,
    pub languages: Vec<String>,
    pub build_system: BuildSystem,
    pub run_candidates: Vec<RunCandidate>,
    pub workspace_members: Vec<String>,
}
#[derive(Debug, Clone, Default)]
pub struct Detection {
    pub projects: Vec<ProjectMetadata>,
    pub warnings: Vec<String>,
    pub truncated: bool,
}
#[derive(Debug)]
pub enum ProjectError {
    Io(std::io::Error),
    InvalidManifest(PathBuf),
    TooLarge(PathBuf),
}
impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::InvalidManifest(p) => write!(f, "Invalid manifest: {}", p.display()),
            Self::TooLarge(p) => write!(f, "Manifest exceeds 1 MiB: {}", p.display()),
        }
    }
}
impl std::error::Error for ProjectError {}
impl From<std::io::Error> for ProjectError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
fn read(path: &Path) -> Result<String, ProjectError> {
    let file = std::fs::File::open(path)?;
    let mut text = String::new();
    file.take(1024 * 1024 + 1).read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 {
        return Err(ProjectError::TooLarge(path.into()));
    }
    Ok(text)
}
fn candidate(label: &str, program: &str, args: Vec<String>, root: &Path) -> RunCandidate {
    RunCandidate {
        label: label.into(),
        program: program.into(),
        arguments: args,
        directory: root.into(),
        requires_workspace_trust: true,
    }
}
pub fn inspect(path: &Path) -> Result<Option<ProjectMetadata>, ProjectError> {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let root = path
        .parent()
        .ok_or_else(|| ProjectError::InvalidManifest(path.into()))?;
    let mut project = ProjectMetadata {
        name: root
            .file_name()
            .unwrap_or(root.as_os_str())
            .to_string_lossy()
            .into_owned(),
        kind: ProjectKind::Rust,
        root: root.into(),
        manifest: path.into(),
        languages: Vec::new(),
        build_system: BuildSystem::Cargo,
        run_candidates: Vec::new(),
        workspace_members: Vec::new(),
    };
    match name {
        "Cargo.toml" => {
            let text = read(path)?;
            let doc: toml::Value =
                toml::from_str(&text).map_err(|_| ProjectError::InvalidManifest(path.into()))?;
            if doc.get("package").is_none() && doc.get("workspace").is_none() {
                return Ok(None);
            }
            if let Some(name) = doc
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(toml::Value::as_str)
            {
                project.name = name.into();
            }
            project.languages.push("Rust".into());
            if let Some(members) = doc
                .get("workspace")
                .and_then(|w| w.get("members"))
                .and_then(toml::Value::as_array)
            {
                project.workspace_members = members
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(String::from)
                    .collect();
            }
            if root.join("src/main.rs").is_file() || doc.get("bin").is_some() {
                project.run_candidates.push(candidate(
                    "Run Rust binary",
                    "cargo",
                    vec!["run".into()],
                    root,
                ));
            }
        }
        "package.json" => {
            let doc: serde_json::Value = serde_json::from_str(&read(path)?)
                .map_err(|_| ProjectError::InvalidManifest(path.into()))?;
            if !doc.is_object() {
                return Err(ProjectError::InvalidManifest(path.into()));
            }
            project.kind = ProjectKind::Node;
            project.build_system = BuildSystem::NodeScripts;
            project.languages = vec!["Node.js".into()];
            if let Some(name) = doc.get("name").and_then(serde_json::Value::as_str) {
                project.name = name.into();
            }
            if doc
                .get("devDependencies")
                .and_then(|v| v.get("typescript"))
                .is_some()
                || root.join("tsconfig.json").is_file()
            {
                project.languages.push("TypeScript".into());
            }
            if let Some(scripts) = doc.get("scripts").and_then(serde_json::Value::as_object) {
                for (name, script) in scripts {
                    if script.is_string() {
                        project.run_candidates.push(candidate(
                            &format!("npm: {name}"),
                            "npm",
                            vec!["run".into(), name.clone()],
                            root,
                        ));
                    }
                }
            }
            if let Some(members) = doc.get("workspaces").and_then(serde_json::Value::as_array) {
                project.workspace_members = members
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(String::from)
                    .collect();
            }
        }
        "pyproject.toml" | "requirements.txt" => {
            project.kind = ProjectKind::Python;
            project.build_system = BuildSystem::Python;
            project.languages = vec!["Python".into()];
            if name == "pyproject.toml" {
                let doc: toml::Value = toml::from_str(&read(path)?)
                    .map_err(|_| ProjectError::InvalidManifest(path.into()))?;
                if let Some(name) = doc
                    .get("project")
                    .and_then(|p| p.get("name"))
                    .and_then(toml::Value::as_str)
                {
                    project.name = name.into();
                }
            }
            if root.join("__main__.py").is_file() {
                project.run_candidates.push(candidate(
                    "Python __main__",
                    "python3",
                    vec!["__main__.py".into()],
                    root,
                ));
            }
        }
        "go.mod" => {
            project.kind = ProjectKind::Go;
            project.build_system = BuildSystem::Go;
            project.languages = vec!["Go".into()];
            let text = read(path)?;
            if let Some(module) = text.lines().find_map(|l| l.trim().strip_prefix("module ")) {
                project.name = module.trim().into();
            }
            project.run_candidates.push(candidate(
                "Run Go package",
                "go",
                vec!["run".into(), ".".into()],
                root,
            ));
        }
        "pom.xml" | "build.gradle" | "build.gradle.kts" => {
            project.kind = ProjectKind::Java;
            project.languages = vec!["Java".into()];
            project.build_system = if name == "pom.xml" {
                BuildSystem::Maven
            } else {
                BuildSystem::Gradle
            };
        }
        "CMakeLists.txt" | "Makefile" => {
            project.kind = ProjectKind::Cpp;
            project.languages = vec!["C/C++".into()];
            project.build_system = if name == "Makefile" {
                BuildSystem::Make
            } else {
                BuildSystem::CMake
            };
        }
        _ if matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("sln" | "csproj")
        ) =>
        {
            project.kind = ProjectKind::DotNet;
            project.languages = vec!["C#".into()];
            project.build_system = BuildSystem::DotNet;
            if path.extension().is_some_and(|s| s == "csproj") {
                project.run_candidates.push(candidate(
                    "Run .NET project",
                    "dotnet",
                    vec!["run".into(), "--project".into(), name.into()],
                    root,
                ));
            }
        }
        _ => return Ok(None),
    }
    Ok(Some(project))
}
pub fn detect(root: &Path, cancel: &AtomicBool) -> Detection {
    let mut result = Detection::default();
    let mut visited = 0;
    let mut walk = ignore::WalkBuilder::new(root);
    walk.hidden(false)
        .follow_links(false)
        .max_depth(Some(16))
        .require_git(false)
        .filter_entry(|e| {
            !matches!(
                e.file_name().to_str(),
                Some(".git" | "target" | "node_modules" | ".venv" | "venv")
            )
        });
    for entry in walk.build() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        visited += 1;
        if visited > 100_000 || result.projects.len() >= 1024 {
            result.truncated = true;
            break;
        }
        match entry {
            Ok(entry) if entry.file_type().is_some_and(|t| t.is_file()) => {
                match inspect(entry.path()) {
                    Ok(Some(project)) => result.projects.push(project),
                    Ok(None) => {}
                    Err(e) => {
                        if result.warnings.len() < 100 {
                            result.warnings.push(e.to_string());
                        }
                    }
                }
            }
            Err(e) if result.warnings.len() < 100 => {
                result.warnings.push(format!("Traversal error: {e}"));
            }
            _ => {}
        }
    }
    result.projects.sort_by(|a, b| a.manifest.cmp(&b.manifest));
    result
}
pub struct Scan {
    pub events: mpsc::Receiver<Detection>,
    cancel: Arc<AtomicBool>,
}
impl Scan {
    pub fn start(root: PathBuf) -> Self {
        let (tx, events) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker = cancel.clone();
        std::thread::spawn(move || {
            let _ = tx.send(detect(&root, &worker));
        });
        Self { events, cancel }
    }
}
impl Drop for Scan {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monorepo_manifest_data_never_executes_scripts() {
        let root = std::env::temp_dir().join(format!("buture-project-{}", std::process::id()));
        std::fs::create_dir_all(root.join("web")).expect("fixture");
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=['web']").expect("rust");
        std::fs::write(root.join("web/package.json"),r#"{"name":"frontend","scripts":{"dev":"touch should-not-exist"},"devDependencies":{"typescript":"*"}}"#).expect("node");
        let result = detect(&root, &AtomicBool::new(false));
        assert_eq!(result.projects.len(), 2);
        let node = result
            .projects
            .iter()
            .find(|p| p.kind == ProjectKind::Node)
            .expect("node");
        assert_eq!(node.run_candidates[0].arguments, vec!["run", "dev"]);
        assert!(!root.join("web/should-not-exist").exists());
        std::fs::write(root.join("web/package.json"), "broken").expect("invalid");
        assert!(inspect(&root.join("web/package.json")).is_err());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
