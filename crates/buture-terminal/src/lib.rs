//! Portable PTY ownership, bounded output and ANSI emulation outside the UI thread.
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};
pub use vt100::{Color, Screen};
#[derive(Debug, Clone)]
pub struct TerminalError(pub String);
impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for TerminalError {}
pub fn shell() -> Result<PathBuf, TerminalError> {
    let variable = if cfg!(windows) { "COMSPEC" } else { "SHELL" };
    if let Some(path) = std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.is_file())
    {
        return Ok(path);
    }
    for path in ["/bin/bash", "/bin/sh"] {
        if Path::new(path).is_file() {
            return Ok(path.into());
        }
    }
    Err(TerminalError(
        "No absolute shell executable detected".into(),
    ))
}
pub enum Event {
    Started { shell: PathBuf, cwd: PathBuf },
    Screen(Box<Screen>),
    Exited(u32),
    Failed(TerminalError),
}
enum Command {
    Input(Vec<u8>),
    Resize(u16, u16),
    Scroll(usize),
    Kill,
}
pub struct Session {
    pub events: mpsc::Receiver<Event>,
    sender: mpsc::SyncSender<Command>,
}
impl Session {
    pub fn start(cwd: PathBuf) -> Self {
        let (sender, commands) = mpsc::sync_channel(64);
        let (tx, events) = mpsc::sync_channel(2);
        std::thread::spawn(move || {
            if let Err(e) = run(cwd, commands, &tx) {
                let _ = tx.send(Event::Failed(e));
            }
        });
        Self { sender, events }
    }
    pub fn input(&self, bytes: Vec<u8>) -> Result<(), TerminalError> {
        if bytes.len() > 1024 * 1024 {
            return Err(TerminalError("Paste exceeds 1 MiB".into()));
        }
        self.sender
            .try_send(Command::Input(bytes))
            .map_err(|_| TerminalError("Terminal input queue unavailable".into()))
    }
    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self
            .sender
            .try_send(Command::Resize(rows.clamp(1, 200), cols.clamp(1, 400)));
    }
    pub fn scroll(&self, offset: usize) {
        let _ = self.sender.try_send(Command::Scroll(offset.min(2000)));
    }
    pub fn kill(&self) {
        let _ = self.sender.try_send(Command::Kill);
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.sender.try_send(Command::Kill);
    }
}
fn run(
    cwd: PathBuf,
    commands: mpsc::Receiver<Command>,
    tx: &mpsc::SyncSender<Event>,
) -> Result<(), TerminalError> {
    let convert = |e: &dyn std::fmt::Display| TerminalError(e.to_string());
    let cwd = std::fs::canonicalize(cwd).map_err(|e| convert(&e))?;
    if !cwd.is_dir() {
        return Err(TerminalError("Terminal cwd must be a directory".into()));
    }
    let executable = shell()?;
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| convert(&e))?;
    let mut builder = CommandBuilder::new(&executable);
    builder.cwd(&cwd);
    builder.env("TERM", "xterm-256color");
    let mut child = pair.slave.spawn_command(builder).map_err(|e| convert(&e))?;
    struct KillOnDrop(Box<dyn portable_pty::ChildKiller + Send + Sync>);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            let _ = self.0.kill();
        }
    }
    let _cleanup = KillOnDrop(child.clone_killer());
    drop(pair.slave);
    let mut writer = pair.master.take_writer().map_err(|e| convert(&e))?;
    let mut reader = pair.master.try_clone_reader().map_err(|e| convert(&e))?;
    let (data_tx, data) = mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let mut bytes = [0; 8192];
        loop {
            match reader.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if data_tx.send(bytes[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let _ = tx.send(Event::Started {
        shell: executable,
        cwd,
    });
    let mut parser = vt100::Parser::new(24, 80, 2000);
    let mut dirty = true;
    let mut last = Instant::now();
    loop {
        match commands.recv_timeout(Duration::from_millis(10)) {
            Ok(Command::Kill) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(());
            }
            Ok(Command::Input(bytes)) => {
                parser.screen_mut().set_scrollback(0);
                writer.write_all(&bytes).map_err(|e| convert(&e))?;
                writer.flush().map_err(|e| convert(&e))?;
                dirty = true;
            }
            Ok(Command::Resize(rows, cols)) => {
                pair.master
                    .resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    })
                    .map_err(|e| convert(&e))?;
                parser.screen_mut().set_size(rows, cols);
                dirty = true;
            }
            Ok(Command::Scroll(offset)) => {
                parser.screen_mut().set_scrollback(offset);
                dirty = true;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        for bytes in data.try_iter().take(16) {
            parser.process(&bytes);
            dirty = true;
        }
        if dirty
            && last.elapsed() >= Duration::from_millis(33)
            && tx
                .try_send(Event::Screen(Box::new(parser.screen().clone())))
                .is_ok()
        {
            dirty = false;
            last = Instant::now();
        }
        if let Some(status) = child.try_wait().map_err(|e| convert(&e))? {
            let _ = tx.send(Event::Screen(Box::new(parser.screen().clone())));
            let _ = tx.send(Event::Exited(status.exit_code()));
            return Ok(());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ansi_state_and_scrollback_are_real() {
        let mut p = vt100::Parser::new(2, 20, 10);
        p.process(b"\x1b[31mred\x1b[0m\r\nnext\r\nlast");
        assert_eq!(
            p.screen().cell(0, 0).expect("cell").fgcolor(),
            Color::Default
        );
        p.screen_mut().set_scrollback(1);
        assert!(p.screen().contents().contains("red"));
    }
    #[test]
    #[cfg(unix)]
    fn pty_accepts_input_resizes_and_reports_exit() {
        let s = Session::start(std::env::temp_dir());
        let deadline = Instant::now();
        let mut sent = false;
        let mut output = String::new();
        loop {
            assert!(deadline.elapsed() < Duration::from_secs(8));
            match s
                .events
                .recv_timeout(Duration::from_secs(2))
                .expect("event")
            {
                Event::Started { .. } => {
                    s.resize(30, 100);
                    s.input(b"printf 'BUTURE_PTY_OK\\n'; exit 0\r".to_vec())
                        .expect("input");
                    sent = true;
                }
                Event::Screen(screen) => output.push_str(&screen.contents()),
                Event::Exited(code) => {
                    assert_eq!(code, 0);
                    break;
                }
                Event::Failed(e) => panic!("{e}"),
            }
        }
        assert!(sent && output.contains("BUTURE_PTY_OK"));
    }
}
