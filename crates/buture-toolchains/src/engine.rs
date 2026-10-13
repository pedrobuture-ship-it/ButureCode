//! Bounded, read-only process probes. No project commands and no shell evaluation.
use crate::Host;
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Compiler,
    Runtime,
    PackageManager,
    BuildTool,
    Debugger,
    Formatter,
    LanguageServer,
    VersionManager,
}
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: &'static str,
    pub role: Role,
    pub arguments: &'static [&'static str],
}
impl ToolSpec {
    pub const fn new(name: &'static str, role: Role) -> Self {
        Self {
            name,
            role,
            arguments: &["--version"],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeState {
    Missing,
    Ready(String),
    Failed(String),
}
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub role: Role,
    pub executable: Option<PathBuf>,
    pub state: ProbeState,
}
#[derive(Debug, Clone)]
pub struct Toolchain {
    pub language: String,
    pub tools: Vec<Tool>,
    pub environments: Vec<PathBuf>,
}
pub trait ToolchainProvider: Send {
    fn language(&self) -> &'static str;
    fn specifications(&self) -> Vec<ToolSpec>;
    fn extra_directories(&self, _host: &Host) -> Vec<PathBuf> {
        Vec::new()
    }
    fn environments(&self, _host: &Host) -> Vec<PathBuf> {
        Vec::new()
    }
    fn detect(&self, host: &Host) -> Toolchain {
        let mut directories = host.path.clone();
        directories.extend(self.extra_directories(host));
        directories.retain(|p| p.is_absolute());
        directories.sort();
        directories.dedup();
        let tools = self
            .specifications()
            .into_iter()
            .flat_map(|spec| {
                let paths = resolve(spec.name, &directories);
                if paths.is_empty() {
                    vec![Tool {
                        name: spec.name.into(),
                        role: spec.role,
                        executable: None,
                        state: ProbeState::Missing,
                    }]
                } else {
                    paths
                        .into_iter()
                        .map(|path| Tool {
                            name: spec.name.into(),
                            role: spec.role,
                            state: probe(&path, spec.arguments, Duration::from_secs(3)),
                            executable: Some(path),
                        })
                        .collect()
                }
            })
            .collect();
        Toolchain {
            language: self.language().into(),
            tools,
            environments: self.environments(host),
        }
    }
}
pub fn resolve(name: &str, directories: &[PathBuf]) -> Vec<PathBuf> {
    let suffixes: &[&str] = if cfg!(target_os = "windows") {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    let mut paths = Vec::new();
    for directory in directories {
        for suffix in suffixes {
            let path = directory.join(format!("{name}{suffix}"));
            if !path.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if std::fs::metadata(&path).map_or(true, |m| m.permissions().mode() & 0o111 == 0) {
                    continue;
                }
            }
            // Preserve aliases such as rustc/rustup; canonicalizing would change multicall behavior.
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    paths
}
pub fn probe(path: &Path, arguments: &[&str], timeout: Duration) -> ProbeState {
    // Windows command scripts require shell evaluation; report them without invoking a shell.
    if cfg!(target_os = "windows")
        && matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("cmd" | "bat")
        )
    {
        return ProbeState::Failed(
            "Command script detected; direct version probe unsupported".into(),
        );
    }
    let directory = match ProbeDirectory::new() {
        Ok(d) => d,
        Err(e) => return ProbeState::Failed(e.to_string()),
    };
    let mut child = match Command::new(path)
        .args(arguments)
        .current_dir(&directory.0)
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env("MISE_AUTO_INSTALL", "false")
        .env("MISE_EXEC_AUTO_INSTALL", "false")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return ProbeState::Failed(e.to_string()),
    };
    let (tx, rx) = mpsc::channel();
    for (stream_index, stream) in [
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
    .enumerate()
    {
        let tx = tx.clone();
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stream.take(16 * 1024).read_to_end(&mut bytes);
            let _ = tx.send((stream_index, bytes));
        });
    }
    drop(tx);
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return ProbeState::Failed(e.to_string());
            }
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return ProbeState::Failed("Version probe timed out".into());
        }
        thread::sleep(Duration::from_millis(15));
    };
    let mut lines: [Option<String>; 2] = [None, None];
    for _ in 0..2 {
        if let Ok((index, bytes)) = rx.recv_timeout(Duration::from_millis(100)) {
            let text = String::from_utf8_lossy(&bytes);
            if let Some(line) = text.lines().find(|s| !s.trim().is_empty()) {
                lines[index] = Some(
                    line.chars()
                        .filter(|c| !c.is_control())
                        .take(300)
                        .collect::<String>(),
                );
            }
        }
    }
    if status.success() {
        match lines.into_iter().flatten().next() {
            Some(version) => ProbeState::Ready(version),
            None => ProbeState::Failed("Tool returned no version".into()),
        }
    } else {
        ProbeState::Failed(format!("Version probe exited with {status}"))
    }
}
struct ProbeDirectory(PathBuf);
impl ProbeDirectory {
    fn new() -> std::io::Result<Self> {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("buture-probe-{}-{sequence}", std::process::id()));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path)?;
        Ok(Self(path))
    }
}
impl Drop for ProbeDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.0);
    }
}

pub fn child_bins(root: PathBuf) -> Vec<PathBuf> {
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path().join("bin"))
        .filter(|p| p.is_dir())
        .take(64)
        .collect()
}
#[derive(Debug)]
pub enum ScanEvent {
    Result(Toolchain),
    Complete,
}
pub struct Scan {
    pub events: mpsc::Receiver<ScanEvent>,
}
impl Scan {
    pub fn start(host: Host, languages: Vec<String>) -> Self {
        let (tx, events) = mpsc::channel();
        thread::spawn(move || {
            let mut workers = Vec::new();
            for provider in crate::providers::registry() {
                let selected = languages.is_empty()
                    || languages.iter().any(|l| {
                        l == provider.language()
                            || (provider.language() == "C/C++"
                                && ["C", "C++"].contains(&l.as_str()))
                            || (provider.language() == "Node.js" && l == "TypeScript")
                    });
                if !selected {
                    continue;
                }
                let tx = tx.clone();
                let host = host.clone();
                workers.push(thread::spawn(move || {
                    let result = provider.detect(&host);
                    let _ = tx.send(ScanEvent::Result(result));
                }));
            }
            for worker in workers {
                let _ = worker.join();
            }
            let _ = tx.send(ScanEvent::Complete);
        });
        Self { events }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_executable_is_not_ready() {
        assert!(matches!(
            probe(
                Path::new("/nonexistent/buture-tool"),
                &["--version"],
                Duration::from_millis(50)
            ),
            ProbeState::Failed(_)
        ));
    }
    #[cfg(unix)]
    #[test]
    fn probe_has_deadline() {
        let state = probe(Path::new("/bin/sleep"), &["2"], Duration::from_millis(30));
        assert_eq!(state, ProbeState::Failed("Version probe timed out".into()));
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn probes_disable_implicit_installation_and_avoid_project_cwd() {
        assert_eq!(
            probe(
                Path::new("/usr/bin/printenv"),
                &["RUSTUP_AUTO_INSTALL"],
                Duration::from_secs(1)
            ),
            ProbeState::Ready("0".into())
        );
        assert_eq!(
            probe(
                Path::new("/usr/bin/printenv"),
                &["MISE_AUTO_INSTALL"],
                Duration::from_secs(1)
            ),
            ProbeState::Ready("false".into())
        );
        match probe(Path::new("/usr/bin/pwd"), &[], Duration::from_secs(1)) {
            ProbeState::Ready(path) => {
                assert!(path.contains("buture-probe-"));
                assert!(!Path::new(&path).exists());
            }
            state => panic!("Unexpected probe: {state:?}"),
        }
    }
}
