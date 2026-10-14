//! Local subprocess framing shared by LSP and DAP. No shell, network or workspace discovery.
pub use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
pub const LIMIT: usize = 16 * 1024 * 1024;
#[derive(Debug)]
pub enum ProtocolError {
    Io(std::io::Error),
    InvalidHeader,
    TooLarge,
    Json(serde_json::Error),
    InvalidExecutable,
    QueueUnavailable,
}
impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Json(e) => write!(f, "Invalid JSON protocol message: {e}"),
            Self::InvalidHeader => write!(f, "Invalid Content-Length framing"),
            Self::TooLarge => write!(f, "Protocol message exceeds 16 MiB"),
            Self::InvalidExecutable => write!(
                f,
                "Protocol executable and cwd must be absolute existing paths"
            ),
            Self::QueueUnavailable => write!(f, "Protocol queue unavailable"),
        }
    }
}
impl std::error::Error for ProtocolError {}
impl From<std::io::Error> for ProtocolError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for ProtocolError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
pub fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, ProtocolError> {
    let mut length = None;
    let mut total = 0;
    loop {
        let mut line = Vec::new();
        let n = reader.by_ref().take(8193).read_until(b'\n', &mut line)?;
        if n == 0 && total == 0 {
            return Ok(None);
        }
        total += n;
        if n == 0 || total > 8192 || !line.ends_with(b"\r\n") {
            return Err(ProtocolError::InvalidHeader);
        }
        if line == b"\r\n" {
            break;
        }
        let text = std::str::from_utf8(&line).map_err(|_| ProtocolError::InvalidHeader)?;
        let (name, value) = text
            .trim()
            .split_once(':')
            .ok_or(ProtocolError::InvalidHeader)?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(ProtocolError::InvalidHeader);
            }
            let size = value
                .trim()
                .parse::<usize>()
                .map_err(|_| ProtocolError::InvalidHeader)?;
            if size > LIMIT {
                return Err(ProtocolError::TooLarge);
            }
            length = Some(size);
        }
    }
    let length = length.ok_or(ProtocolError::InvalidHeader)?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}
pub fn write_message(writer: &mut impl Write, message: &Value) -> Result<(), ProtocolError> {
    let bytes = serde_json::to_vec(message)?;
    if bytes.len() > LIMIT {
        return Err(ProtocolError::TooLarge);
    }
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}
#[derive(Clone, Debug)]
pub struct Spec {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}
pub enum Event {
    Ready,
    Message(Value),
    Failed(ProtocolError),
    Exited(Option<i32>),
}
pub struct Process {
    pub events: mpsc::Receiver<Event>,
    sender: mpsc::SyncSender<Value>,
    stop: Arc<AtomicBool>,
}
impl Process {
    pub fn start(spec: Spec) -> Self {
        let (sender, requests) = mpsc::sync_channel(64);
        let (tx, events) = mpsc::sync_channel(64);
        let stop = Arc::new(AtomicBool::new(false));
        let cancel = stop.clone();
        std::thread::spawn(move || {
            if let Err(e) = run(spec, requests, &tx, &cancel) {
                let _ = tx.send(Event::Failed(e));
            }
        });
        Self {
            events,
            sender,
            stop,
        }
    }
    pub fn send(&self, value: Value) -> Result<(), ProtocolError> {
        self.sender
            .try_send(value)
            .map_err(|_| ProtocolError::QueueUnavailable)
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        self.stop();
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
fn run(
    spec: Spec,
    requests: mpsc::Receiver<Value>,
    tx: &mpsc::SyncSender<Event>,
    stop: &AtomicBool,
) -> Result<(), ProtocolError> {
    if !spec.executable.is_absolute()
        || !spec.executable.is_file()
        || !spec.cwd.is_absolute()
        || !spec.cwd.is_dir()
        || spec.args.iter().map(String::len).sum::<usize>() > 65536
    {
        return Err(ProtocolError::InvalidExecutable);
    }
    let mut child = ChildGuard(
        Command::new(spec.executable)
            .args(spec.args)
            .current_dir(spec.cwd)
            .env("RUSTUP_AUTO_INSTALL", "0")
            .env("MISE_AUTO_INSTALL", "false")
            .env("CARGO_NET_OFFLINE", "true")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let mut input = child
        .0
        .stdin
        .take()
        .ok_or(ProtocolError::InvalidExecutable)?;
    let stdout = child
        .0
        .stdout
        .take()
        .ok_or(ProtocolError::InvalidExecutable)?;
    let stderr = child
        .0
        .stderr
        .take()
        .ok_or(ProtocolError::InvalidExecutable)?;
    std::thread::spawn(move || {
        let mut reader = stderr;
        let mut bytes = [0; 8192];
        while reader.read(&mut bytes).is_ok_and(|n| n > 0) {}
    });
    let (write_tx, write_rx) = mpsc::sync_channel::<Value>(64);
    let (fail_tx, fail_rx) = mpsc::channel();
    let writer_fail = fail_tx.clone();
    std::thread::spawn(move || {
        while let Ok(value) = write_rx.recv() {
            if let Err(e) = write_message(&mut input, &value) {
                let _ = writer_fail.send(e);
                break;
            }
        }
    });
    let (read_tx, read_rx) = mpsc::sync_channel(64);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_message(&mut reader) {
                Ok(Some(message)) => {
                    if read_tx.send(message).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = fail_tx.send(e);
                    break;
                }
            }
        }
    });
    let _ = tx.send(Event::Ready);
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        if let Ok(e) = fail_rx.try_recv() {
            return Err(e);
        }
        match requests.recv_timeout(Duration::from_millis(10)) {
            Ok(message) => write_tx
                .try_send(message)
                .map_err(|_| ProtocolError::QueueUnavailable)?,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        for message in read_rx.try_iter().take(64) {
            if tx.send(Event::Message(message)).is_err() {
                return Ok(());
            }
        }
        if let Some(status) = child.0.try_wait()? {
            for message in read_rx.try_iter() {
                let _ = tx.send(Event::Message(message));
            }
            let _ = tx.send(Event::Exited(status.code()));
            return Ok(());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_handles_unicode_and_rejects_duplicate_oversized_headers() {
        let value = serde_json::json!({"text":"🦀 café"});
        let mut bytes = Vec::new();
        write_message(&mut bytes, &value).expect("write");
        assert_eq!(
            read_message(&mut std::io::Cursor::new(bytes)).expect("read"),
            Some(value)
        );
        for bytes in [
            b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_vec(),
            format!("Content-Length: {}\r\n\r\n", LIMIT + 1).into_bytes(),
            vec![b'a'; 8193],
        ] {
            assert!(read_message(&mut std::io::Cursor::new(bytes)).is_err());
        }
    }
    #[test]
    #[cfg(unix)]
    fn local_process_roundtrips_framed_json() {
        let python = PathBuf::from("/usr/bin/python3");
        if !python.is_file() {
            return;
        }
        let code = "import sys,json\nn=int(sys.stdin.buffer.readline().split(b':')[1]);sys.stdin.buffer.readline();v=json.loads(sys.stdin.buffer.read(n));b=json.dumps(v).encode();sys.stdout.buffer.write(b'Content-Length: '+str(len(b)).encode()+b'\\r\\n\\r\\n'+b);sys.stdout.buffer.flush();sys.stdin.buffer.read(1)";
        let process = Process::start(Spec {
            executable: python,
            args: vec!["-u".into(), "-c".into(), code.into()],
            cwd: std::env::temp_dir(),
        });
        process
            .send(serde_json::json!({"id":1,"result":"ok"}))
            .expect("send");
        loop {
            match process
                .events
                .recv_timeout(Duration::from_secs(3))
                .expect("event")
            {
                Event::Message(value) => {
                    assert_eq!(value["result"], "ok");
                    break;
                }
                Event::Ready => {}
                Event::Failed(e) => panic!("{e}"),
                Event::Exited(code) => panic!("early exit {code:?}"),
            }
        }
    }
}
