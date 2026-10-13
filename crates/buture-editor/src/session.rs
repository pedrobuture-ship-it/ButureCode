//! Layout contains paths and presentation only. It cannot define commands or escape a workspace.
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Debug, Clone)]
pub struct Tab {
    pub path: PathBuf,
    pub pinned: bool,
}
#[derive(Debug, Clone, Default)]
pub struct Group {
    pub tabs: Vec<Tab>,
    pub active: Option<PathBuf>,
}
#[derive(Debug, Clone)]
pub struct Layout {
    pub workspace: PathBuf,
    pub orientation: Orientation,
    pub groups: Vec<Group>,
}
#[derive(Debug)]
pub enum SessionError {
    Io(std::io::Error),
    Invalid,
    Conflict,
}
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Invalid => write!(f, "Invalid editor session; existing data was preserved"),
            Self::Conflict => write!(
                f,
                "Editor session changed externally; refusing to overwrite it"
            ),
        }
    }
}
impl std::error::Error for SessionError {}
impl From<std::io::Error> for SessionError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl Layout {
    pub fn parse(text: &str) -> Result<Self, SessionError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|_| SessionError::Invalid)?;
        if value["version"] != 1 {
            return Err(SessionError::Invalid);
        }
        let workspace = PathBuf::from(value["workspace"].as_str().ok_or(SessionError::Invalid)?);
        if !workspace.is_absolute() {
            return Err(SessionError::Invalid);
        }
        let orientation = match value["orientation"].as_str() {
            Some("horizontal") => Orientation::Horizontal,
            Some("vertical") => Orientation::Vertical,
            _ => return Err(SessionError::Invalid),
        };
        let groups = value["groups"].as_array().ok_or(SessionError::Invalid)?;
        if groups.is_empty() || groups.len() > 4 {
            return Err(SessionError::Invalid);
        }
        let mut output = Vec::new();
        for group in groups {
            let tabs = group["tabs"].as_array().ok_or(SessionError::Invalid)?;
            if tabs.len() > 128 {
                return Err(SessionError::Invalid);
            }
            let mut entries = Vec::new();
            for tab in tabs {
                let relative = PathBuf::from(tab["path"].as_str().ok_or(SessionError::Invalid)?);
                if relative.as_os_str().is_empty()
                    || relative
                        .components()
                        .any(|c| !matches!(c, std::path::Component::Normal(_)))
                {
                    return Err(SessionError::Invalid);
                }
                entries.push(Tab {
                    path: workspace.join(relative),
                    pinned: tab["pinned"].as_bool().ok_or(SessionError::Invalid)?,
                });
            }
            let active = group["active"]
                .as_u64()
                .and_then(|i| entries.get(i as usize))
                .map(|t| t.path.clone());
            output.push(Group {
                tabs: entries,
                active,
            });
        }
        Ok(Self {
            workspace,
            orientation,
            groups: output,
        })
    }
    pub fn encode(&self) -> Result<String, SessionError> {
        let groups = self
            .groups
            .iter()
            .map(|g| {
                let tabs = g
                    .tabs
                    .iter()
                    .filter_map(|t| {
                        t.path.strip_prefix(&self.workspace).ok().map(
                            |p| serde_json::json!({"path":p.to_string_lossy(),"pinned":t.pinned}),
                        )
                    })
                    .collect::<Vec<_>>();
                let active = g
                    .active
                    .as_ref()
                    .and_then(|path| g.tabs.iter().position(|t| &t.path == path));
                serde_json::json!({"tabs":tabs,"active":active})
            })
            .collect::<Vec<_>>();
        serde_json::to_string_pretty(&serde_json::json!({"version":1,"workspace":self.workspace.to_string_lossy(),"orientation":match self.orientation{Orientation::Horizontal=>"horizontal",Orientation::Vertical=>"vertical"},"groups":groups})).map_err(|_|SessionError::Invalid)
    }
}
pub enum Request {
    Load(PathBuf),
    Save(Layout),
    Flush(mpsc::Sender<()>),
    Stop,
}
pub enum Event {
    Loaded(Option<Layout>),
    Failed(SessionError),
}
pub struct Store {
    pub events: mpsc::Receiver<Event>,
    sender: mpsc::Sender<Request>,
    worker: Option<thread::JoinHandle<()>>,
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, SessionError> {
    match std::fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
            if bytes.len() > 1024 * 1024 {
                return Err(SessionError::Invalid);
            }
            Ok(Some(bytes))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
impl Store {
    pub fn start(path: PathBuf) -> Self {
        let (sender, rx) = mpsc::channel();
        let (tx, events) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut previous = None;
            let mut loaded = false;
            while let Ok(request) = rx.recv() {
                match request {
                    Request::Load(root) => {
                        loaded = false;
                        let result = (|| {
                            previous = read(&path)?;
                            let layout = previous
                                .as_ref()
                                .map(|b| {
                                    std::str::from_utf8(b)
                                        .map_err(|_| SessionError::Invalid)
                                        .and_then(Layout::parse)
                                })
                                .transpose()?;
                            loaded = true;
                            Ok(layout.filter(|l| l.workspace == root))
                        })();
                        let _ = tx.send(match result {
                            Ok(layout) => Event::Loaded(layout),
                            Err(e) => Event::Failed(e),
                        });
                    }
                    Request::Save(layout) => {
                        let result = (|| {
                            if !loaded {
                                return Err(SessionError::Invalid);
                            }
                            if read(&path)? != previous {
                                return Err(SessionError::Conflict);
                            }
                            let text = layout.encode()?;
                            if let Some(parent) = path.parent() {
                                std::fs::create_dir_all(parent)?;
                            }
                            let temporary =
                                path.with_extension(format!("tmp-{}", std::process::id()));
                            let mut file = std::fs::OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(&temporary)?;
                            file.write_all(text.as_bytes())?;
                            file.sync_all()?;
                            drop(file);
                            std::fs::rename(temporary, &path)?;
                            previous = Some(text.into_bytes());
                            Ok(())
                        })();
                        if let Err(e) = result {
                            let _ = tx.send(Event::Failed(e));
                        }
                    }
                    Request::Flush(ack) => {
                        let _ = ack.send(());
                    }
                    Request::Stop => break,
                }
            }
        });
        Self {
            events,
            sender,
            worker: Some(worker),
        }
    }
    pub fn load(&self, root: PathBuf) {
        let _ = self.sender.send(Request::Load(root));
    }
    pub fn save(&self, layout: Layout) {
        let _ = self.sender.send(Request::Save(layout));
    }
    pub fn flush(&self) {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(Request::Flush(tx));
        let _ = rx.recv_timeout(std::time::Duration::from_secs(2));
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = self.sender.send(Request::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout_roundtrips_and_rejects_path_escape() {
        let l = Layout {
            workspace: PathBuf::from(if cfg!(windows) {
                "C:\\project"
            } else {
                "/project"
            }),
            orientation: Orientation::Vertical,
            groups: vec![Group {
                tabs: vec![Tab {
                    path: PathBuf::from(if cfg!(windows) {
                        "C:\\project\\src\\main.rs"
                    } else {
                        "/project/src/main.rs"
                    }),
                    pinned: true,
                }],
                active: None,
            }],
        };
        let text = l.encode().expect("encode");
        let result = Layout::parse(&text).expect("parse");
        assert_eq!(result.orientation, Orientation::Vertical);
        assert!(result.groups[0].tabs[0].pinned);
        assert!(
            Layout::parse(
                &text
                    .replace("src/main.rs", "../escape")
                    .replace("src\\\\main.rs", "..\\\\escape")
            )
            .is_err()
        );
    }
    #[test]
    fn store_preserves_external_layout_changes() {
        let root = std::env::temp_dir().join(format!("buture-session-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture");
        let path = root.join("session.json");
        let store = Store::start(path.clone());
        store.load(root.clone());
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Loaded(None))
        ));
        let layout = Layout {
            workspace: root.clone(),
            orientation: Orientation::Horizontal,
            groups: vec![Group::default()],
        };
        store.save(layout.clone());
        store.flush();
        assert!(Layout::parse(&std::fs::read_to_string(&path).expect("saved")).is_ok());
        std::fs::write(&path, "owner edit").expect("external");
        store.save(layout);
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Failed(SessionError::Conflict))
        ));
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "owner edit");
        drop(store);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
