//! Tasks are data; only a reviewed, explicitly confirmed Plan can execute.
use buture_project::{BuildSystem, Detection};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: PathBuf,
    pub dependencies: Vec<String>,
    pub run: bool,
}
#[derive(Debug)]
pub enum TaskError {
    Io(std::io::Error),
    OutsideWorkspace,
    MissingDependency(String),
    DependencyCycle,
    InvalidTask,
    Cancelled,
    Failed { task: String, code: Option<i32> },
}
impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::OutsideWorkspace => write!(f, "Task directory is outside the reviewed workspace"),
            Self::MissingDependency(id) => write!(f, "Missing task dependency: {id}"),
            Self::DependencyCycle => write!(f, "Task dependency cycle"),
            Self::InvalidTask => write!(f, "Invalid task data"),
            Self::Cancelled => write!(f, "Task cancelled (direct child terminated)"),
            Self::Failed { task, code } => {
                write!(f, "Task {task} failed with exit status {code:?}")
            }
        }
    }
}
impl std::error::Error for TaskError {}
impl From<std::io::Error> for TaskError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
fn task(root: &Path, program: &str, args: &[&str], label: &str, run: bool) -> Task {
    Task {
        id: format!("{}:{program}:{}", root.display(), args.join(" ")),
        label: label.into(),
        command: program.into(),
        args: args.iter().map(|s| (*s).into()).collect(),
        env: BTreeMap::new(),
        cwd: root.into(),
        dependencies: vec![],
        run,
    }
}
pub fn detect(projects: &Detection) -> Vec<Task> {
    let mut tasks = vec![];
    for project in &projects.projects {
        let root = &project.root;
        match project.build_system {
            BuildSystem::Cargo => {
                for (arg, label) in [
                    ("build", "Cargo Build"),
                    ("test", "Cargo Test"),
                    ("check", "Cargo Check"),
                ] {
                    tasks.push(task(root, "cargo", &[arg], label, false));
                }
            }
            BuildSystem::NodeScripts => {
                let manager = if root.join("pnpm-lock.yaml").exists() {
                    "pnpm"
                } else {
                    "npm"
                };
                if let Ok(file) = std::fs::File::open(&project.manifest) {
                    let mut bytes = Vec::new();
                    if file.take(1024 * 1024 + 1).read_to_end(&mut bytes).is_ok()
                        && bytes.len() <= 1024 * 1024
                        && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
                        && let Some(scripts) = value["scripts"].as_object()
                    {
                        for name in scripts.keys().filter(|name| name.len() <= 256).take(1024) {
                            tasks.push(task(
                                root,
                                manager,
                                &["run", name],
                                &format!("{manager}: {name}"),
                                ["start", "dev", "serve", "preview"].contains(&name.as_str()),
                            ));
                        }
                    }
                }
            }
            BuildSystem::Make => {
                tasks.push(task(root, "make", &[], "Make Build", false));
                tasks.push(task(
                    root,
                    "make",
                    &["test"],
                    "Make Test (target may be absent)",
                    false,
                ));
            }
            BuildSystem::CMake => {
                let configure = task(
                    root,
                    "cmake",
                    &["-S", ".", "-B", "build"],
                    "CMake Configure",
                    false,
                );
                let mut build = task(root, "cmake", &["--build", "build"], "CMake Build", false);
                build.dependencies.push(configure.id.clone());
                tasks.extend([configure, build]);
            }
            BuildSystem::Maven => {
                for (arg, label) in [("compile", "Maven Build"), ("test", "Maven Test")] {
                    tasks.push(task(root, "mvn", &[arg], label, false));
                }
            }
            BuildSystem::Gradle => {
                for (arg, label) in [("build", "Gradle Build"), ("test", "Gradle Test")] {
                    tasks.push(task(root, "gradle", &[arg], label, false));
                }
            }
            BuildSystem::Go => {
                tasks.push(task(root, "go", &["build", "./..."], "Go Build", false));
                tasks.push(task(root, "go", &["test", "./..."], "Go Test", false));
            }
            _ => {}
        }
        if project.build_system == BuildSystem::Python {
            for file in ["main.py", "app.py", "manage.py"] {
                if root.join(file).is_file() {
                    tasks.push(task(
                        root,
                        if cfg!(windows) { "python" } else { "python3" },
                        &[file],
                        &format!("{} · Python {file}", project.name),
                        true,
                    ));
                }
            }
        }
        if project.build_system == BuildSystem::Gradle
            && let Ok(text) = run::manifest_text(&project.manifest)
            && text.len() <= 1024 * 1024
            && text.contains("application")
        {
            tasks.push(task(
                root,
                "gradle",
                &["run"],
                &format!("{} · Gradle Run", project.name),
                true,
            ));
        }
        if project.build_system == BuildSystem::Maven
            && let Ok(text) = run::manifest_text(&project.manifest)
            && text.len() <= 1024 * 1024
            && text.contains("exec-maven-plugin")
        {
            tasks.push(task(
                root,
                "mvn",
                &["exec:java"],
                &format!("{} · Maven Run", project.name),
                true,
            ));
        }
        if project.build_system == BuildSystem::Make
            && let Ok(text) = run::manifest_text(&project.manifest)
            && text.len() <= 1024 * 1024
            && text.lines().any(|l| l.starts_with("run:"))
        {
            tasks.push(task(
                root,
                "make",
                &["run"],
                &format!("{} · Make Run", project.name),
                true,
            ));
        }
        if project.build_system == BuildSystem::Cargo {
            let inferred = run::rust_targets(project);
            if !inferred.is_empty() {
                tasks.extend(inferred);
                continue;
            }
        }
        if [BuildSystem::CMake, BuildSystem::Make].contains(&project.build_system)
            && project.name.len() <= 128
            && project
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ['_', '-'].contains(&c))
        {
            let file = if cfg!(windows) {
                format!("{}.exe", project.name)
            } else {
                project.name.clone()
            };
            for folder in ["build", "."] {
                let binary = root.join(folder).join(&file);
                if binary.is_file() {
                    tasks.push(task(
                        root,
                        &binary.to_string_lossy(),
                        &[],
                        &format!("{} · Run configured binary", project.name),
                        true,
                    ));
                }
            }
        }
        if project.build_system == BuildSystem::NodeScripts {
            continue;
        }
        for candidate in &project.run_candidates {
            let mut entry = task(
                &candidate.directory,
                &candidate.program,
                &candidate
                    .arguments
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                &candidate.label,
                true,
            );
            entry.label = format!("{} · {}", project.name, entry.label);
            if !tasks.iter().any(|t| t.id == entry.id) {
                tasks.push(entry);
            }
        }
    }
    tasks.truncate(4096);
    tasks
}
#[derive(Debug, Clone)]
pub struct Plan {
    pub tasks: Vec<Task>,
    pub root: PathBuf,
}
pub struct Confirmed(Plan);
impl Plan {
    pub fn review(selected: &str, tasks: &[Task], root: &Path) -> Result<Self, TaskError> {
        let root = std::fs::canonicalize(root)?;
        let map: BTreeMap<_, _> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();
        let mut ordered = vec![];
        let mut pending = BTreeSet::new();
        let mut done = BTreeSet::new();
        fn visit(
            id: &str,
            map: &BTreeMap<&str, &Task>,
            root: &Path,
            pending: &mut BTreeSet<String>,
            done: &mut BTreeSet<String>,
            ordered: &mut Vec<Task>,
        ) -> Result<(), TaskError> {
            if done.contains(id) {
                return Ok(());
            }
            if pending.len() >= 256 {
                return Err(TaskError::InvalidTask);
            }
            if !pending.insert(id.into()) {
                return Err(TaskError::DependencyCycle);
            }
            let task = map
                .get(id)
                .ok_or_else(|| TaskError::MissingDependency(id.into()))?;
            let cwd = std::fs::canonicalize(&task.cwd)?;
            if !cwd.starts_with(root) || !cwd.is_dir() {
                return Err(TaskError::OutsideWorkspace);
            }
            if task.command.is_empty()
                || task.command.contains('\0')
                || task.args.iter().any(|arg| arg.contains('\0'))
                || task
                    .env
                    .keys()
                    .any(|key| key.is_empty() || key.contains(['=', '\0']))
                || ordered.len() >= 256
            {
                return Err(TaskError::InvalidTask);
            }
            for dependency in &task.dependencies {
                visit(dependency, map, root, pending, done, ordered)?;
            }
            let mut task = (*task).clone();
            task.cwd = cwd;
            ordered.push(task);
            pending.remove(id);
            done.insert(id.into());
            Ok(())
        }
        visit(selected, &map, &root, &mut pending, &mut done, &mut ordered)?;
        Ok(Self {
            tasks: ordered,
            root,
        })
    }
    pub fn confirm(self) -> Confirmed {
        Confirmed(self)
    }
}
pub enum Event {
    Started(String),
    Output {
        stderr: bool,
        text: String,
    },
    Finished {
        result: Result<(), TaskError>,
        dropped_chunks: usize,
    },
}
pub struct Execution {
    pub events: mpsc::Receiver<Event>,
    cancel: Arc<AtomicBool>,
}
impl Execution {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Drop for Execution {
    fn drop(&mut self) {
        self.cancel();
    }
}
impl Confirmed {
    pub fn execute(self) -> Execution {
        let (tx, events) = mpsc::sync_channel(128);
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        std::thread::spawn(move || {
            let dropped = Arc::new(AtomicUsize::new(0));
            let result = execute(self.0, &tx, &stop, &dropped);
            let _ = tx.send(Event::Finished {
                result,
                dropped_chunks: dropped.load(Ordering::Relaxed),
            });
        });
        Execution { events, cancel }
    }
}
fn output(
    mut reader: impl Read + Send + 'static,
    stderr: bool,
    tx: mpsc::SyncSender<Event>,
    dropped: Arc<AtomicUsize>,
) -> mpsc::Receiver<()> {
    let (complete, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = [0; 8192];
        loop {
            match reader.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let event = Event::Output {
                        stderr,
                        text: String::from_utf8_lossy(&bytes[..n]).into_owned(),
                    };
                    match tx.try_send(event) {
                        Ok(()) => {}
                        Err(mpsc::TrySendError::Full(_)) => {
                            dropped.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => break,
                    }
                }
            }
        }
        let _ = complete.send(());
    });
    receiver
}
fn execute(
    plan: Plan,
    tx: &mpsc::SyncSender<Event>,
    cancel: &AtomicBool,
    dropped: &Arc<AtomicUsize>,
) -> Result<(), TaskError> {
    for task in plan.tasks {
        if cancel.load(Ordering::Relaxed) {
            return Err(TaskError::Cancelled);
        }
        let cwd = std::fs::canonicalize(&task.cwd)?;
        if !cwd.starts_with(&plan.root) {
            return Err(TaskError::OutsideWorkspace);
        }
        let _ = tx.send(Event::Started(task.label.clone()));
        let mut child = Command::new(&task.command)
            .args(&task.args)
            .envs(&task.env)
            .env("RUSTUP_AUTO_INSTALL", "0")
            .env("MISE_AUTO_INSTALL", "false")
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut streams = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            streams.push(output(stdout, false, tx.clone(), dropped.clone()));
        }
        if let Some(stderr) = child.stderr.take() {
            streams.push(output(stderr, true, tx.clone(), dropped.clone()));
        }
        let status = loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(TaskError::Cancelled);
            }
            if let Some(status) = child.try_wait()? {
                break status;
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        for stream in streams {
            if stream.recv_timeout(Duration::from_millis(250)).is_err() {
                dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
        if !status.success() {
            return Err(TaskError::Failed {
                task: task.label,
                code: status.code(),
            });
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_orders_dependencies_and_rejects_cycles_and_escape() {
        let root = std::env::temp_dir();
        let a = task(&root, "echo", &["a"], "a", false);
        let mut b = task(&root, "echo", &["b"], "b", false);
        b.dependencies.push(a.id.clone());
        let mut entries = vec![a.clone(), b.clone()];
        let p = Plan::review(&b.id, &entries, &root).expect("plan");
        assert_eq!(p.tasks[0].id, a.id);
        entries[0].dependencies.push(b.id.clone());
        assert!(matches!(
            Plan::review(&b.id, &entries, &root),
            Err(TaskError::DependencyCycle)
        ));
        entries[0].dependencies.clear();
        entries[0].cwd = std::env::current_dir().expect("cwd");
        assert!(matches!(
            Plan::review(&a.id, &entries, &root),
            Err(TaskError::OutsideWorkspace)
        ));
    }
    #[test]
    #[cfg(unix)]
    fn explicitly_confirmed_plan_executes_and_reports_failure() {
        let root = std::env::temp_dir();
        let a = task(&root, "/bin/sh", &["-c", "exit 7"], "fixture", false);
        let execution = Plan::review(&a.id, std::slice::from_ref(&a), &root)
            .expect("review")
            .confirm()
            .execute();
        loop {
            if let Event::Finished { result, .. } = execution
                .events
                .recv_timeout(Duration::from_secs(3))
                .expect("event")
            {
                assert!(matches!(
                    result,
                    Err(TaskError::Failed { code: Some(7), .. })
                ));
                break;
            }
        }
    }
}

pub mod run;
