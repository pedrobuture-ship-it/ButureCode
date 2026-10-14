//! Formatter registry: stdin/stdout only, explicit confirmation, no write-in-place flags.
use buture_editor::{Buffer, Edit, LineEnding, TextSnapshot};
use std::{
    io::{Read, Write},
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Rustfmt,
    ClangFormat,
    Ruff,
    Black,
    Prettier,
    Gofmt,
}
impl Kind {
    pub const ALL: [Self; 6] = [
        Self::Rustfmt,
        Self::ClangFormat,
        Self::Ruff,
        Self::Black,
        Self::Prettier,
        Self::Gofmt,
    ];
    pub fn executable(self) -> &'static str {
        match self {
            Self::Rustfmt => "rustfmt",
            Self::ClangFormat => "clang-format",
            Self::Ruff => "ruff",
            Self::Black => "black",
            Self::Prettier => "prettier",
            Self::Gofmt => "gofmt",
        }
    }
    pub fn infer(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rustfmt),
            "c" | "h" | "cpp" | "hpp" | "cc" => Some(Self::ClangFormat),
            "py" => Some(Self::Ruff),
            "go" => Some(Self::Gofmt),
            "js" | "jsx" | "ts" | "tsx" | "json" | "html" | "css" | "md" | "yaml" | "yml" => {
                Some(Self::Prettier)
            }
            _ => None,
        }
    }
}
#[derive(Debug)]
pub enum FormatError {
    Io(std::io::Error),
    Unsupported,
    MissingTool,
    OutsideWorkspace,
    TooLarge,
    InvalidSelection,
    Timeout,
    Failed { code: Option<i32>, message: String },
    Encoding,
    EmptyOutput,
}
impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Unsupported => write!(f, "No formatter registered for this document"),
            Self::MissingTool => write!(f, "Formatter executable not found; no tool was installed"),
            Self::OutsideWorkspace => {
                write!(f, "Formatter source/cwd must be inside the reviewed root")
            }
            Self::TooLarge => write!(f, "Formatter limit: 8 MiB input / 16 MiB output"),
            Self::InvalidSelection => write!(
                f,
                "Select a nonempty, complete fragment; CRLF boundaries cannot be split"
            ),
            Self::Timeout => write!(f, "Formatter timed out after 20 seconds"),
            Self::Failed { code, message } => write!(f, "Formatter exit {code:?}: {message}"),
            Self::Encoding => write!(f, "Formatter returned invalid UTF-8"),
            Self::EmptyOutput => write!(
                f,
                "Formatter returned empty output for nonempty source; no edits applied"
            ),
        }
    }
}
impl std::error::Error for FormatError {}
impl From<std::io::Error> for FormatError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
pub fn resolve(kind: Kind, root: &Path) -> Option<PathBuf> {
    if kind == Kind::Prettier {
        let local = root.join("node_modules/.bin/prettier");
        if local.is_file() {
            return Some(local);
        }
    }
    let host = buture_toolchains::Host::detect();
    if kind == Kind::Rustfmt {
        let rustup = std::env::var_os("RUSTUP_HOME")
            .map(PathBuf::from)
            .or_else(|| host.home.as_ref().map(|h| h.join(".rustup")));
        if let Some(rustup) = rustup
            && let Ok(entries) = std::fs::read_dir(rustup.join("toolchains"))
        {
            for entry in entries.flatten().take(128) {
                if entry.file_name().to_string_lossy().starts_with("stable-") {
                    let path = entry.path().join("bin").join(if cfg!(windows) {
                        "rustfmt.exe"
                    } else {
                        "rustfmt"
                    });
                    if path.is_file() {
                        return Some(path);
                    }
                }
            }
        }
    }
    buture_toolchains::engine::resolve(kind.executable(), &host.path)
        .into_iter()
        .next()
}
fn edition(root: &Path, path: &Path) -> String {
    let mut directory = path.parent();
    while let Some(folder) = directory.filter(|p| p.starts_with(root)) {
        if let Ok(file) = std::fs::File::open(folder.join("Cargo.toml")) {
            let mut text = String::new();
            if file.take(1024 * 1024 + 1).read_to_string(&mut text).is_ok()
                && text.len() <= 1024 * 1024
                && let Ok(doc) = toml::from_str::<toml::Value>(&text)
                && let Some(value) = doc
                    .get("package")
                    .and_then(|p| p.get("edition"))
                    .and_then(toml::Value::as_str)
                    .or_else(|| {
                        doc.get("workspace")
                            .and_then(|w| w.get("package"))
                            .and_then(|p| p.get("edition"))
                            .and_then(toml::Value::as_str)
                    })
                    .filter(|s| ["2015", "2018", "2021", "2024"].contains(s))
            {
                return value.into();
            }
        }
        directory = folder.parent();
    }
    "2021".into()
}
#[derive(Debug, Clone)]
pub struct Plan {
    pub snapshot: TextSnapshot,
    pub range: Range<usize>,
    pub kind: Kind,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub root: PathBuf,
    input: String,
    line_ending: LineEnding,
}
pub struct Confirmed(Plan);
impl Plan {
    pub fn review(
        buffer: Buffer,
        selection: Option<Range<usize>>,
        kind: Option<Kind>,
        executable: Option<PathBuf>,
        root: &Path,
    ) -> Result<Self, FormatError> {
        let root = std::fs::canonicalize(root)?;
        let path = buffer.path.as_ref().ok_or(FormatError::OutsideWorkspace)?;
        if !path.is_absolute()
            || !path.starts_with(&root)
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || !path
                .parent()
                .and_then(|p| std::fs::canonicalize(p).ok())
                .is_some_and(|p| p.starts_with(&root))
        {
            return Err(FormatError::OutsideWorkspace);
        }
        let kind = kind
            .or_else(|| Kind::infer(path))
            .ok_or(FormatError::Unsupported)?;
        let executable = executable
            .or_else(|| resolve(kind, &root))
            .filter(|p| p.is_absolute() && p.is_file())
            .ok_or(FormatError::MissingTool)?;
        let range = selection.clone().unwrap_or(0..buffer.len_chars());
        if range.start > range.end
            || range.end > buffer.len_chars()
            || selection.is_some() && range.is_empty()
        {
            return Err(FormatError::InvalidSelection);
        }
        for boundary in [range.start, range.end] {
            if boundary > 0
                && boundary < buffer.len_chars()
                && buffer.rope().char(boundary - 1) == '\r'
                && buffer.rope().char(boundary) == '\n'
            {
                return Err(FormatError::InvalidSelection);
            }
        }
        let input = buffer.rope().slice(range.clone()).to_string();
        if input.len() > 8 * 1024 * 1024 {
            return Err(FormatError::TooLarge);
        }
        let input = input.replace("\r\n", "\n");
        let filename = path.to_string_lossy().into_owned();
        let args = match kind {
            Kind::Rustfmt => vec![
                "--emit".into(),
                "stdout".into(),
                "--edition".into(),
                edition(&root, path),
                "--config".into(),
                "skip_children=true".into(),
            ],
            Kind::ClangFormat => vec![format!("--assume-filename={filename}")],
            Kind::Ruff => vec![
                "format".into(),
                "--stdin-filename".into(),
                filename,
                "-".into(),
            ],
            Kind::Black => vec![
                "--quiet".into(),
                "--stdin-filename".into(),
                filename,
                "-".into(),
            ],
            Kind::Prettier => vec!["--stdin-filepath".into(), filename],
            Kind::Gofmt => vec![],
        };
        Ok(Self {
            snapshot: buffer.snapshot(),
            range,
            kind,
            executable,
            args,
            root,
            input,
            line_ending: buffer.line_ending,
        })
    }
    pub fn confirm(self) -> Confirmed {
        Confirmed(self)
    }
}
pub struct Formatted {
    pub snapshot: TextSnapshot,
    pub edit: Edit,
}
impl Confirmed {
    pub fn execute(self) -> mpsc::Receiver<Result<Formatted, FormatError>> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(run(self.0));
        });
        rx
    }
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
fn capture(
    mut reader: impl Read + Send + 'static,
    limit: usize,
) -> mpsc::Receiver<Result<Vec<u8>, FormatError>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let mut bytes = vec![];
            reader
                .by_ref()
                .take(limit as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > limit {
                return Err(FormatError::TooLarge);
            }
            Ok(bytes)
        })();
        let _ = tx.send(result);
    });
    rx
}
fn run(plan: Plan) -> Result<Formatted, FormatError> {
    let mut child = ChildGuard(
        Command::new(&plan.executable)
            .args(&plan.args)
            .current_dir(&plan.root)
            .env("RUSTUP_AUTO_INSTALL", "0")
            .env("MISE_AUTO_INSTALL", "false")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let mut input = child.0.stdin.take().ok_or(FormatError::Encoding)?;
    let data = plan.input.clone();
    let (input_tx, input_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = input_tx.send(input.write_all(data.as_bytes()));
    });
    let stdout = capture(
        child.0.stdout.take().ok_or(FormatError::Encoding)?,
        16 * 1024 * 1024,
    );
    let stderr = capture(child.0.stderr.take().ok_or(FormatError::Encoding)?, 65536);
    let began = Instant::now();
    let status = loop {
        if began.elapsed() > Duration::from_secs(20) {
            return Err(FormatError::Timeout);
        }
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let input_result = input_rx
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| FormatError::Timeout)?;
    let out = stdout
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| FormatError::Timeout)??;
    let err = stderr
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| FormatError::Timeout)??;
    if !status.success() {
        return Err(FormatError::Failed {
            code: status.code(),
            message: String::from_utf8_lossy(&err).chars().take(2048).collect(),
        });
    }
    input_result?;
    let mut text = String::from_utf8(out).map_err(|_| FormatError::Encoding)?;
    if !plan.input.trim().is_empty() && text.trim().is_empty() {
        return Err(FormatError::EmptyOutput);
    }
    text = text.replace("\r\n", "\n");
    if plan.line_ending == LineEnding::CrLf {
        text = text.replace('\n', "\r\n");
    }
    Ok(Formatted {
        snapshot: plan.snapshot,
        edit: Edit {
            range: plan.range,
            text,
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_cannot_split_crlf_or_escape_workspace() {
        let root = std::env::temp_dir();
        let mut buffer = Buffer::from_text("fn main() {}\r\n");
        buffer.path = Some(root.join("main.rs"));
        let executable = if cfg!(unix) {
            Some(PathBuf::from("/bin/true"))
        } else {
            None
        };
        if executable.is_none() {
            return;
        }
        assert!(matches!(
            Plan::review(
                buffer.clone(),
                Some(0..13),
                Some(Kind::Rustfmt),
                executable.clone(),
                &root
            ),
            Err(FormatError::InvalidSelection)
        ));
        buffer.path = Some(PathBuf::from("/outside.rs"));
        assert!(matches!(
            Plan::review(buffer, None, Some(Kind::Rustfmt), executable, &root),
            Err(FormatError::OutsideWorkspace)
        ));
    }
    #[test]
    #[cfg(unix)]
    fn real_rustfmt_returns_text_and_never_writes_files() {
        let root = std::env::temp_dir().join(format!("buture-format-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture");
        let Some(executable) = resolve(Kind::Rustfmt, &root) else {
            std::fs::remove_dir_all(root).expect("cleanup");
            return;
        };
        let path = root.join("main.rs");
        let text = "mod child;fn main( ){let café=1;}\r\n";
        std::fs::write(&path, text).expect("source");
        let child = root.join("child.rs");
        std::fs::write(&child, "fn    child (){}").expect("child");
        let buffer = Buffer::open(&path).expect("buffer");
        let output = Plan::review(buffer, None, Some(Kind::Rustfmt), Some(executable), &root)
            .expect("review")
            .confirm()
            .execute()
            .recv_timeout(Duration::from_secs(25))
            .expect("event")
            .expect("format");
        assert!(output.edit.text.contains("fn main()"));
        assert!(output.edit.text.contains("\r\n"));
        assert_eq!(std::fs::read_to_string(path).expect("disk"), text);
        assert_eq!(
            std::fs::read_to_string(child).expect("child"),
            "fn    child (){}"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
