//! Native Git through bounded argv-only subprocesses. No auto network, hooks or author invention.
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
const LIMIT: usize = 16 * 1024 * 1024;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
#[derive(Debug)]
pub enum GitError {
    Io(std::io::Error),
    Unavailable,
    NotRepository,
    InvalidPath,
    SensitivePath,
    InvalidRef,
    InvalidMessage,
    NoIdentity,
    Conflict,
    TooLarge,
    Timeout,
    Failed { code: Option<i32>, message: String },
}
impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Unavailable => write!(f, "Git executable not found"),
            Self::NotRepository => write!(f, "Not a Git working tree"),
            Self::InvalidPath => write!(f, "Git path must stay relative to the repository"),
            Self::SensitivePath => write!(
                f,
                "Sensitive paths cannot be staged/committed by this workflow"
            ),
            Self::InvalidRef => write!(f, "Invalid branch/remote name"),
            Self::InvalidMessage => write!(
                f,
                "Commit message must be nonempty UTF-8 text, at most 64 KiB"
            ),
            Self::NoIdentity => write!(f, "Configure your Git name and email before committing"),
            Self::Conflict => write!(
                f,
                "HEAD or index changed after review; refresh before retrying"
            ),
            Self::TooLarge => write!(f, "Git output/patch exceeds 16 MiB"),
            Self::Timeout => write!(
                f,
                "Git operation timed out; remote effects may need inspection"
            ),
            Self::Failed { code, message } => write!(f, "Git exit {code:?}: {message}"),
        }
    }
}
impl std::error::Error for GitError {}
impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
#[derive(Debug, Clone)]
pub struct Repository {
    pub root: PathBuf,
    executable: PathBuf,
}
#[derive(Debug, Clone)]
pub struct Change {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub index: char,
    pub worktree: char,
}
impl Change {
    pub fn staged(&self) -> bool {
        self.index != ' ' && self.index != '?'
    }
    pub fn changed(&self) -> bool {
        self.worktree != ' ' || self.index == '?'
    }
}
#[derive(Debug, Clone)]
pub struct Status {
    pub head: Option<String>,
    pub branch: String,
    pub changes: Vec<Change>,
    pub branches: Vec<String>,
    pub remotes: Vec<String>,
    pub partial: bool,
}
fn decode_path(bytes: &[u8]) -> Result<PathBuf, GitError> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(OsString::from_vec(bytes.to_vec()).into())
    }
    #[cfg(not(unix))]
    {
        Ok(std::str::from_utf8(bytes)
            .map_err(|_| GitError::InvalidPath)?
            .into())
    }
}
fn path_bytes(path: &Path) -> Result<Vec<u8>, GitError> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(path.as_os_str().as_bytes().to_vec())
    }
    #[cfg(not(unix))]
    {
        Ok(path
            .to_str()
            .ok_or(GitError::InvalidPath)?
            .as_bytes()
            .to_vec())
    }
}
pub fn sensitive(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy().to_lowercase();
        s == ".aws"
            || s == ".ssh"
            || s == "credentials"
            || s == "id_rsa"
            || s == "id_ed25519"
            || s == ".env"
            || (s.starts_with(".env.") && s != ".env.example")
    })
}
pub fn validate_path(path: &Path) -> Result<(), GitError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path.components().any(|c| c.as_os_str() == ".git")
    {
        return Err(GitError::InvalidPath);
    }
    Ok(())
}
fn redact(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            if let Some(scheme) = word.find("://")
                && let Some(at) = word[scheme + 3..].find('@')
            {
                format!(
                    "{}[redacted]{}",
                    &word[..scheme + 3],
                    &word[scheme + 3 + at..]
                )
            } else {
                word.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(2048)
        .collect()
}
struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
fn read_output(
    mut reader: impl Read + Send + 'static,
) -> mpsc::Receiver<Result<Vec<u8>, GitError>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let mut bytes = vec![];
            reader
                .by_ref()
                .take(LIMIT as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > LIMIT {
                return Err(GitError::TooLarge);
            }
            Ok(bytes)
        })();
        let _ = tx.send(result);
    });
    rx
}
fn execute(
    executable: &Path,
    root: &Path,
    args: &[OsString],
    input: Option<Vec<u8>>,
) -> Result<Vec<u8>, GitError> {
    let mut command = Command::new(executable);
    command
        .args([
            "--no-pager",
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.quotePath=false",
        ])
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = ChildGuard(command.spawn()?);
    if let Some(input) = input
        && let Some(mut stdin) = child.0.stdin.take()
    {
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let out = read_output(child.0.stdout.take().ok_or(GitError::Unavailable)?);
    let err = read_output(child.0.stderr.take().ok_or(GitError::Unavailable)?);
    let began = Instant::now();
    let status = loop {
        if began.elapsed() > Duration::from_secs(60) {
            return Err(GitError::Timeout);
        }
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = out
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| GitError::Timeout)??;
    let stderr = err
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| GitError::Timeout)??;
    if !status.success() {
        return Err(GitError::Failed {
            code: status.code(),
            message: redact(&String::from_utf8_lossy(&stderr)),
        });
    }
    Ok(stdout)
}
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
fn lines(bytes: Vec<u8>) -> Vec<String> {
    String::from_utf8_lossy(&bytes)
        .lines()
        .map(String::from)
        .take(10000)
        .collect()
}
fn oid(bytes: Vec<u8>) -> Result<String, GitError> {
    let value = String::from_utf8(bytes)
        .map_err(|_| GitError::InvalidRef)?
        .trim()
        .to_owned();
    if ![40, 64].contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(GitError::InvalidRef);
    }
    Ok(value)
}
impl Repository {
    pub fn discover(folder: &Path) -> Result<Self, GitError> {
        let folder = std::fs::canonicalize(folder)?;
        let host = buture_toolchains::Host::detect();
        let executable = buture_toolchains::engine::resolve("git", &host.path)
            .into_iter()
            .next()
            .ok_or(GitError::Unavailable)?;
        let bytes = execute(
            &executable,
            &folder,
            &args(&["rev-parse", "--show-toplevel"]),
            None,
        )
        .map_err(|error| match error {
            GitError::Failed { ref message, .. }
                if message.to_lowercase().contains("not a git repository") =>
            {
                GitError::NotRepository
            }
            other => other,
        })?;
        let bytes = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
        let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
        let root = std::fs::canonicalize(decode_path(bytes)?)?;
        Ok(Self { root, executable })
    }
    pub(crate) fn read(&self, values: &[&str]) -> Result<Vec<u8>, GitError> {
        execute(&self.executable, &self.root, &args(values), None)
    }
    pub fn head(&self) -> Result<Option<String>, GitError> {
        match self.read(&["rev-parse", "--verify", "HEAD"]) {
            Ok(bytes) => Ok(Some(oid(bytes)?)),
            Err(GitError::Failed {
                code: Some(128), ..
            })
            | Err(GitError::Failed { code: Some(1), .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub fn status(&self) -> Result<Status, GitError> {
        let raw = self.read(&[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignore-submodules=all",
        ])?;
        let mut fields = raw.split(|b| *b == 0).filter(|f| !f.is_empty());
        let mut changes = vec![];
        let mut partial = false;
        while let Some(field) = fields.next() {
            if field.len() < 4 || field[2] != b' ' {
                return Err(GitError::InvalidPath);
            }
            let path = decode_path(&field[3..])?;
            validate_path(&path)?;
            let old_path = if b"RC".contains(&field[0]) || b"RC".contains(&field[1]) {
                Some(decode_path(fields.next().ok_or(GitError::InvalidPath)?)?)
            } else {
                None
            };
            changes.push(Change {
                path,
                old_path,
                index: field[0] as char,
                worktree: field[1] as char,
            });
            if changes.len() >= 10000 {
                partial = fields.next().is_some();
                break;
            }
        }
        let head = self.head()?;
        let branch = self
            .read(&["symbolic-ref", "--short", "-q", "HEAD"])
            .ok()
            .map(|v| String::from_utf8_lossy(&v).trim().to_owned())
            .unwrap_or_else(|| {
                head.as_ref()
                    .map(|h| format!("Detached {}", &h[..7]))
                    .unwrap_or_else(|| "Unborn HEAD".into())
            });
        let branches =
            lines(self.read(&["for-each-ref", "--format=%(refname:short)", "refs/heads"])?);
        let remotes = lines(self.read(&["remote"])?);
        Ok(Status {
            head,
            branch,
            changes,
            branches,
            remotes,
            partial,
        })
    }
    pub fn diff(&self, path: Option<&Path>, staged: bool) -> Result<Vec<u8>, GitError> {
        let mut values = args(&["diff", "--no-ext-diff", "--no-textconv", "--binary"]);
        if staged {
            values.push("--cached".into());
        }
        values.push("--".into());
        if let Some(path) = path {
            validate_path(path)?;
            values.push(path.as_os_str().into());
        }
        execute(&self.executable, &self.root, &values, None)
    }
    pub fn identity(&self) -> Result<(String, String), GitError> {
        let name = self
            .read(&["config", "--get", "user.name"])
            .ok()
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
            .filter(|s| !s.is_empty());
        let email = self
            .read(&["config", "--get", "user.email"])
            .ok()
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
            .filter(|s| !s.is_empty());
        name.zip(email).ok_or(GitError::NoIdentity)
    }
    fn check_branch(&self, name: &str) -> Result<(), GitError> {
        if name.is_empty() || name.starts_with('-') || name.contains(['\0', '\n', '\r', '{', '}']) {
            return Err(GitError::InvalidRef);
        }
        self.read(&["check-ref-format", "--branch", name])
            .map_err(|_| GitError::InvalidRef)?;
        Ok(())
    }
    pub fn review(&self, operation: Operation) -> Result<Plan, GitError> {
        let head = self.head()?;
        let reference = self
            .read(&["symbolic-ref", "-q", "HEAD"])
            .ok()
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
            .unwrap_or_else(|| "HEAD".into());
        let mut tree = None;
        let mut staged = None;
        match &operation {
            Operation::Stage(paths) | Operation::Unstage(paths) => {
                if paths.is_empty() || paths.len() > 10000 {
                    return Err(GitError::InvalidPath);
                }
                for path in paths {
                    validate_path(path)?;
                    if matches!(operation, Operation::Stage(_)) && sensitive(path) {
                        return Err(GitError::SensitivePath);
                    }
                }
            }
            Operation::Commit(message) => {
                if message.trim().is_empty() || message.len() > 65536 || message.contains('\0') {
                    return Err(GitError::InvalidMessage);
                }
                self.identity()?;
                let status = self.status()?;
                if status
                    .changes
                    .iter()
                    .filter(|c| c.staged())
                    .any(|c| sensitive(&c.path))
                {
                    return Err(GitError::SensitivePath);
                }
                let diff = self.diff(None, true)?;
                if diff.is_empty() {
                    return Err(GitError::InvalidMessage);
                }
                staged = Some(diff);
                tree = Some(oid(self.read(&["write-tree"])?)?);
            }
            Operation::Checkout(name) | Operation::CreateBranch(name) => self.check_branch(name)?,
            Operation::Fetch(remote) | Operation::Pull(remote) | Operation::Push(remote) => {
                if remote.is_empty()
                    || remote.starts_with('-')
                    || !remote
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || ['_', '-', '.', '/'].contains(&c))
                    || !lines(self.read(&["remote"])?).contains(remote)
                {
                    return Err(GitError::InvalidRef);
                }
            }
        }
        Ok(Plan {
            repository: self.clone(),
            operation,
            head,
            reference,
            tree,
            staged,
        })
    }
}
#[derive(Debug, Clone)]
pub enum Operation {
    Stage(Vec<PathBuf>),
    Unstage(Vec<PathBuf>),
    Commit(String),
    Checkout(String),
    CreateBranch(String),
    Fetch(String),
    Pull(String),
    Push(String),
}
#[derive(Debug, Clone)]
pub struct Plan {
    pub repository: Repository,
    pub operation: Operation,
    head: Option<String>,
    reference: String,
    tree: Option<String>,
    staged: Option<Vec<u8>>,
}
pub struct Confirmed(Plan);
impl Plan {
    pub fn confirm(self) -> Confirmed {
        Confirmed(self)
    }
    pub fn staged_preview(&self) -> Option<&[u8]> {
        self.staged.as_deref()
    }
}
struct MessageFile {
    directory: PathBuf,
    path: PathBuf,
}
impl Drop for MessageFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.directory);
    }
}
impl MessageFile {
    fn new(message: &str) -> Result<Self, GitError> {
        let directory = std::env::temp_dir().join(format!(
            "buture-git-message-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory)?;
        let path = directory.join("message.txt");
        let owned = Self { directory, path };
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&owned.path)?;
        file.write_all(message.as_bytes())?;
        file.sync_all()?;
        Ok(owned)
    }
}
impl Confirmed {
    pub fn execute(self) -> mpsc::Receiver<Result<String, GitError>> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(apply(self.0));
        });
        rx
    }
}
fn paths_input(paths: &[PathBuf]) -> Result<Vec<u8>, GitError> {
    let mut input = vec![];
    for path in paths {
        input.extend(path_bytes(path)?);
        input.push(0);
        if input.len() > LIMIT {
            return Err(GitError::TooLarge);
        }
    }
    Ok(input)
}
fn apply(plan: Plan) -> Result<String, GitError> {
    let repo = &plan.repository;
    let head = repo.head()?;
    if head != plan.head {
        return Err(GitError::Conflict);
    }
    let values = match &plan.operation {
        Operation::Stage(paths) => {
            return execute(
                &repo.executable,
                &repo.root,
                &args(&["add", "--pathspec-from-file=-", "--pathspec-file-nul"]),
                Some(paths_input(paths)?),
            )
            .map(|_| "Staged paths".into());
        }
        Operation::Unstage(paths) => {
            let command = if plan.head.is_some() {
                args(&[
                    "restore",
                    "--staged",
                    "--pathspec-from-file=-",
                    "--pathspec-file-nul",
                ])
            } else {
                args(&[
                    "rm",
                    "--cached",
                    "--ignore-unmatch",
                    "--pathspec-from-file=-",
                    "--pathspec-file-nul",
                ])
            };
            return execute(
                &repo.executable,
                &repo.root,
                &command,
                Some(paths_input(paths)?),
            )
            .map(|_| "Unstaged paths; worktree preserved".into());
        }
        Operation::Commit(message) => {
            repo.identity()?;
            if Some(repo.diff(None, true)?) != plan.staged {
                return Err(GitError::Conflict);
            }
            let tree = plan.tree.as_ref().ok_or(GitError::Conflict)?;
            if oid(repo.read(&["write-tree"])?)? != *tree {
                return Err(GitError::Conflict);
            }
            let file = MessageFile::new(message)?;
            let mut values = args(&["commit-tree", tree]);
            if let Some(head) = &plan.head {
                values.extend(args(&["-p", head]));
            }
            if repo
                .read(&["config", "--bool", "--get", "commit.gpgsign"])
                .ok()
                .is_some_and(|b| String::from_utf8_lossy(&b).trim() == "true")
            {
                values.push("-S".into());
            }
            values.push("-F".into());
            values.push(file.path.as_os_str().into());
            let commit = oid(execute(&repo.executable, &repo.root, &values, None)?)?;
            let reference = repo
                .read(&["symbolic-ref", "-q", "HEAD"])
                .ok()
                .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
                .unwrap_or_else(|| "HEAD".into());
            if reference != plan.reference {
                return Err(GitError::Conflict);
            }
            let expected = plan.head.clone().unwrap_or_else(|| "0".repeat(tree.len()));
            repo.read(&[
                "update-ref",
                "-m",
                "Buture Code reviewed commit",
                &plan.reference,
                &commit,
                &expected,
            ])?;
            return Ok(format!("Commit {} on {}", &commit[..7], plan.reference));
        }
        Operation::Checkout(name) => args(&["switch", name]),
        Operation::CreateBranch(name) => args(&["branch", name]),
        Operation::Fetch(remote) => args(&["fetch", remote]),
        Operation::Pull(remote) => args(&["pull", "--ff-only", "--no-rebase", remote]),
        Operation::Push(remote) => args(&["push", remote, "HEAD"]),
    };
    execute(&repo.executable, &repo.root, &values, None).map(|bytes| {
        let output = redact(&String::from_utf8_lossy(&bytes));
        if output.is_empty() {
            "Git operation completed".into()
        } else {
            output
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_and_sensitive_files_are_guarded() {
        assert!(validate_path(Path::new("../escape")).is_err());
        assert!(validate_path(Path::new(".git/config")).is_err());
        assert!(sensitive(Path::new("folder/.env.local")));
        assert!(!sensitive(Path::new(".env.example")));
        assert_eq!(
            redact("fatal https://user:secret@example.com/repo"),
            "fatal https://[redacted]@example.com/repo"
        );
    }
    #[test]
    fn native_repo_stage_unstage_commit_and_stale_review_preserve_worktree() {
        let Ok(owner) = Repository::discover(&std::env::current_dir().expect("cwd")) else {
            return;
        };
        let Ok((name, email)) = owner.identity() else {
            return;
        };
        let root = std::env::temp_dir().join(format!("buture-git-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture");
        execute(
            &owner.executable,
            &root,
            &args(&["init", "-b", "main"]),
            None,
        )
        .expect("init");
        execute(
            &owner.executable,
            &root,
            &args(&["config", "user.name", &name]),
            None,
        )
        .expect("configured owner");
        execute(
            &owner.executable,
            &root,
            &args(&["config", "user.email", &email]),
            None,
        )
        .expect("configured email");
        let repo = Repository::discover(&root).expect("repo");
        let path = PathBuf::from("café file.txt");
        std::fs::write(root.join(&path), "one\n").expect("source");
        let wait = |plan: Plan| {
            plan.confirm()
                .execute()
                .recv_timeout(Duration::from_secs(5))
                .expect("event")
        };
        wait(
            repo.review(Operation::Stage(vec![path.clone()]))
                .expect("review"),
        )
        .expect("stage");
        assert!(repo.status().expect("status").changes[0].staged());
        wait(
            repo.review(Operation::Unstage(vec![path.clone()]))
                .expect("review"),
        )
        .expect("unstage");
        assert!(root.join(&path).is_file());
        wait(
            repo.review(Operation::Stage(vec![path.clone()]))
                .expect("review"),
        )
        .expect("stage");
        let old = repo
            .review(Operation::Commit("[test] fixture commit".into()))
            .expect("commit review");
        std::fs::write(root.join(&path), "two\n").expect("owner edit");
        wait(
            repo.review(Operation::Stage(vec![path.clone()]))
                .expect("review"),
        )
        .expect("stage");
        assert!(matches!(wait(old), Err(GitError::Conflict)));
        wait(
            repo.review(Operation::Commit("[test] fixture commit".into()))
                .expect("review"),
        )
        .expect("commit");
        assert!(repo.status().expect("status").changes.is_empty());
        assert_eq!(repo.identity().expect("author"), (name, email));
        std::fs::write(root.join(&path), "owner worktree edit\n").expect("edit");
        wait(
            repo.review(Operation::Stage(vec![path.clone()]))
                .expect("review"),
        )
        .expect("stage");
        wait(
            repo.review(Operation::Unstage(vec![path.clone()]))
                .expect("review"),
        )
        .expect("unstage");
        assert_eq!(
            std::fs::read_to_string(root.join(path)).expect("file"),
            "owner worktree edit\n"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
