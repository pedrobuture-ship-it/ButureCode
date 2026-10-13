//! Workspace filesystem actor. Only expanded directories are scanned, on a worker thread.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub directory: bool,
    pub depth: usize,
    pub expanded: bool,
}
#[derive(Debug, Clone)]
pub enum Operation {
    NewFile(String),
    NewFolder(String),
    Rename(String),
    Duplicate(String),
    CopyTo(PathBuf),
    Delete,
}
#[derive(Debug)]
pub enum Request {
    Open(PathBuf),
    Toggle(PathBuf),
    Refresh,
    Collapse,
    Read(PathBuf),
    Mutate(PathBuf, Operation),
    Reveal(PathBuf),
}
#[derive(Debug)]
pub enum Event {
    Tree {
        root: PathBuf,
        entries: Vec<Entry>,
        branch: Option<String>,
    },
    Preview {
        path: PathBuf,
        text: String,
        line_offsets: Vec<usize>,
    },
    Error(String),
}
#[derive(Debug)]
pub enum WorkspaceError {
    Io(io::Error),
    OutsideRoot,
    InvalidName,
    Unsupported,
    TooLarge,
}
impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::OutsideRoot => write!(f, "Path escapes the workspace or traverses a symlink"),
            Self::InvalidName => write!(f, "Enter a single file or folder name"),
            Self::Unsupported => write!(f, "Operation is not supported for this entry"),
            Self::TooLarge => write!(f, "Preview is limited to UTF-8 files up to 1 MiB"),
        }
    }
}
impl std::error::Error for WorkspaceError {}
impl From<io::Error> for WorkspaceError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

pub struct Workspace {
    requests: Sender<Request>,
    pub events: Receiver<Event>,
}
impl Default for Workspace {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let (events_tx, events) = mpsc::channel();
        thread::spawn(move || {
            let mut root = None;
            let mut expanded = BTreeSet::new();
            let mut previous = Vec::new();
            let mut previous_branch = None;
            loop {
                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(request) => {
                        let result: Result<(), WorkspaceError> = (|| {
                            match request {
                                Request::Open(path) => {
                                    let path = fs::canonicalize(path)?;
                                    if !path.is_dir() {
                                        return Err(WorkspaceError::Unsupported);
                                    }
                                    expanded.clear();
                                    expanded.insert(path.clone());
                                    root = Some(path);
                                    previous.clear();
                                }
                                Request::Toggle(path) => {
                                    if !expanded.remove(&path) {
                                        expanded.insert(path);
                                    }
                                }
                                Request::Collapse => {
                                    expanded.clear();
                                    if let Some(r) = &root {
                                        expanded.insert(r.clone());
                                    }
                                }
                                Request::Refresh => {
                                    previous.clear();
                                }
                                Request::Read(path) => {
                                    let r = root.as_ref().ok_or(WorkspaceError::OutsideRoot)?;
                                    let path = checked(r, &path)?;
                                    let file = fs::File::open(&path)?;
                                    use std::io::Read;
                                    let mut data = Vec::new();
                                    file.take(1024 * 1024 + 1).read_to_end(&mut data)?;
                                    if data.len() > 1024 * 1024 {
                                        return Err(WorkspaceError::TooLarge);
                                    }
                                    let text = String::from_utf8(data)
                                        .map_err(|_| WorkspaceError::TooLarge)?;
                                    let line_offsets = std::iter::once(0)
                                        .chain(
                                            text.bytes()
                                                .enumerate()
                                                .filter_map(|(i, b)| (b == b'\n').then_some(i + 1)),
                                        )
                                        .collect();
                                    let _ = events_tx.send(Event::Preview {
                                        path,
                                        text,
                                        line_offsets,
                                    });
                                }
                                Request::Mutate(path, op) => {
                                    mutate(
                                        root.as_ref().ok_or(WorkspaceError::OutsideRoot)?,
                                        &path,
                                        op,
                                    )?;
                                    previous.clear();
                                }
                                Request::Reveal(path) => {
                                    let path = checked(
                                        root.as_ref().ok_or(WorkspaceError::OutsideRoot)?,
                                        &path,
                                    )?;
                                    let target = if path.is_dir() {
                                        path
                                    } else {
                                        path.parent().ok_or(WorkspaceError::OutsideRoot)?.to_owned()
                                    };
                                    let command = if cfg!(target_os = "windows") {
                                        "explorer"
                                    } else if cfg!(target_os = "macos") {
                                        "open"
                                    } else {
                                        "xdg-open"
                                    };
                                    std::process::Command::new(command).arg(target).spawn()?;
                                }
                            }
                            Ok(())
                        })();
                        if let Err(e) = result {
                            let _ = events_tx.send(Event::Error(e.to_string()));
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                if let Some(root) = &root {
                    match tree(root, &expanded) {
                        Ok(entries) if entries != previous || branch(root) != previous_branch => {
                            previous = entries.clone();
                            previous_branch = branch(root);
                            if events_tx
                                .send(Event::Tree {
                                    root: root.clone(),
                                    entries,
                                    branch: previous_branch.clone(),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            let _ = events_tx.send(Event::Error(e.to_string()));
                        }
                    }
                }
            }
        });
        Self {
            requests: tx,
            events,
        }
    }
}
impl Workspace {
    pub fn send(&self, request: Request) {
        let _ = self.requests.send(request);
    }
}

fn branch(root: &Path) -> Option<String> {
    use std::io::Read;
    let file = fs::File::open(root.join(".git/HEAD")).ok()?;
    let mut text = String::new();
    file.take(4096).read_to_string(&mut text).ok()?;
    let value = text.trim();
    if let Some(branch) = value.strip_prefix("ref: refs/heads/") {
        Some(
            branch
                .chars()
                .filter(|c| !c.is_control())
                .take(100)
                .collect(),
        )
    } else if value.len() >= 7 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(format!("Detached {}", &value[..7]))
    } else {
        None
    }
}

fn tree(root: &Path, expanded: &BTreeSet<PathBuf>) -> Result<Vec<Entry>, WorkspaceError> {
    let mut rows = Vec::new();
    let mut pending = vec![(root.to_owned(), 0)];
    while let Some((path, depth)) = pending.pop() {
        let metadata = fs::symlink_metadata(&path)?;
        let directory = metadata.is_dir() && !metadata.file_type().is_symlink();
        let open = directory && expanded.contains(&path);
        rows.push(Entry {
            path: path.clone(),
            directory,
            depth,
            expanded: open,
        });
        if open {
            let mut children = BTreeMap::new();
            for child in fs::read_dir(path)? {
                let child = child?;
                let kind = child.file_type()?;
                children.insert((!kind.is_dir(), child.file_name()), child.path());
            }
            pending.extend(children.into_values().rev().map(|p| (p, depth + 1)));
        }
    }
    Ok(rows)
}
fn checked(root: &Path, path: &Path) -> Result<PathBuf, WorkspaceError> {
    if !path.starts_with(root) {
        return Err(WorkspaceError::OutsideRoot);
    }
    let relative = path
        .strip_prefix(root)
        .map_err(|_| WorkspaceError::OutsideRoot)?;
    let mut cursor = root.to_owned();
    for component in relative.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(WorkspaceError::OutsideRoot);
        }
        cursor.push(component);
        if fs::symlink_metadata(&cursor)?.file_type().is_symlink() {
            return Err(WorkspaceError::OutsideRoot);
        }
    }
    let canonical = fs::canonicalize(path)?;
    if !canonical.starts_with(root) {
        return Err(WorkspaceError::OutsideRoot);
    }
    Ok(canonical)
}
fn name(value: &str) -> Result<&str, WorkspaceError> {
    if value.is_empty() || value == "." || value == ".." || value.contains(['/', '\\', '\0']) {
        return Err(WorkspaceError::InvalidName);
    }
    Ok(value)
}
fn copy(source: &Path, target: &Path) -> Result<(), WorkspaceError> {
    if target.exists() {
        return Err(WorkspaceError::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Destination already exists",
        )));
    }
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return Err(WorkspaceError::Unsupported);
    }
    if metadata.is_dir() {
        fs::create_dir(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy(&entry.path(), &target.join(entry.file_name()))?;
        }
    } else {
        use std::io::Write;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        let mut input = fs::File::open(source)?;
        io::copy(&mut input, &mut output)?;
        output.flush()?;
    }
    Ok(())
}
fn mutate(root: &Path, path: &Path, operation: Operation) -> Result<(), WorkspaceError> {
    let source = checked(root, path)?;
    match operation {
        Operation::NewFile(value) => {
            if !source.is_dir() {
                return Err(WorkspaceError::Unsupported);
            }
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(source.join(name(&value)?))?;
        }
        Operation::NewFolder(value) => {
            if !source.is_dir() {
                return Err(WorkspaceError::Unsupported);
            }
            fs::create_dir(source.join(name(&value)?))?;
        }
        Operation::Rename(ref value) | Operation::Duplicate(ref value) => {
            if source == root {
                return Err(WorkspaceError::Unsupported);
            }
            let target = source.with_file_name(name(value)?);
            if target.exists() {
                return Err(WorkspaceError::Io(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Destination already exists",
                )));
            }
            if matches!(operation, Operation::Rename(_)) {
                fs::rename(source, target)?;
            } else {
                copy(&source, &target)?;
            }
        }
        Operation::CopyTo(destination) => {
            let parent = checked(
                root,
                destination.parent().ok_or(WorkspaceError::OutsideRoot)?,
            )?;
            let target = parent.join(destination.file_name().ok_or(WorkspaceError::InvalidName)?);
            if target.starts_with(&source) {
                return Err(WorkspaceError::Unsupported);
            }
            copy(&source, &target)?;
        }
        Operation::Delete => {
            if source == root {
                return Err(WorkspaceError::Unsupported);
            }
            if source.is_dir() {
                fs::remove_dir_all(source)?;
            } else {
                fs::remove_file(source)?;
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lazy_tree_and_safe_mutations() {
        let root = std::env::temp_dir().join(format!("buture-workspace-{}", std::process::id()));
        fs::create_dir_all(root.join("nested/deep")).expect("fixture");
        let root = fs::canonicalize(root).expect("root");
        let expanded = BTreeSet::from([root.clone()]);
        assert_eq!(tree(&root, &expanded).expect("tree").len(), 2);
        assert!(mutate(&root, &root, Operation::NewFile("../escape".into())).is_err());
        mutate(&root, &root, Operation::NewFile("hello.rs".into())).expect("create");
        assert!(mutate(&root, &root, Operation::NewFile("hello.rs".into())).is_err());
        mutate(
            &root,
            &root.join("hello.rs"),
            Operation::Duplicate("copy.rs".into()),
        )
        .expect("copy");
        assert!(mutate(&root, &root, Operation::Delete).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn watcher_and_preview_are_live() {
        let root = std::env::temp_dir().join(format!("buture-watch-{}", std::process::id()));
        fs::create_dir_all(&root).expect("fixture");
        let root = fs::canonicalize(root).expect("root");
        let workspace = Workspace::default();
        workspace.send(Request::Open(root.clone()));
        assert!(matches!(
            workspace.events.recv_timeout(Duration::from_secs(4)),
            Ok(Event::Tree { .. })
        ));
        fs::write(root.join("live.txt"), "hello").expect("external write");
        match workspace
            .events
            .recv_timeout(Duration::from_secs(4))
            .expect("watch event")
        {
            Event::Tree { entries, .. } => {
                assert!(entries.iter().any(|e| e.path.ends_with("live.txt")))
            }
            event => panic!("Unexpected event: {event:?}"),
        }
        workspace.send(Request::Read(root.join("live.txt")));
        match workspace
            .events
            .recv_timeout(Duration::from_secs(4))
            .expect("preview")
        {
            Event::Preview { text, .. } => assert_eq!(text, "hello"),
            event => panic!("Unexpected event: {event:?}"),
        }
        drop(workspace);
        fs::remove_dir_all(root).expect("cleanup");
    }
    #[cfg(unix)]
    #[test]
    fn symlink_operations_are_rejected() {
        let root = std::env::temp_dir().join(format!("buture-symlink-{}", std::process::id()));
        fs::create_dir_all(&root).expect("fixture");
        let root = fs::canonicalize(root).expect("root");
        std::os::unix::fs::symlink(std::env::temp_dir(), root.join("escape")).expect("symlink");
        assert!(checked(&root, &root.join("escape")).is_err());
        assert!(mutate(&root, &root.join("escape"), Operation::Delete).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
