//! Smart Run chooses observed candidates; preference is selection, never execution trust.
use crate::Task;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
};
#[derive(Debug, PartialEq, Eq)]
pub enum Choice {
    Unavailable,
    Selected(String),
    Ambiguous(Vec<String>),
}
pub fn choose(tasks: &[Task], remembered: Option<&str>) -> Choice {
    let candidates: Vec<_> = tasks.iter().filter(|t| t.run).collect();
    if let Some(id) = remembered
        && candidates.iter().any(|t| t.id == id)
    {
        return Choice::Selected(id.into());
    }
    match candidates.as_slice() {
        [] => Choice::Unavailable,
        [single] => Choice::Selected(single.id.clone()),
        many => Choice::Ambiguous(many.iter().map(|t| t.id.clone()).collect()),
    }
}
#[derive(Debug)]
pub enum PreferenceError {
    Io(std::io::Error),
    Invalid,
    Conflict,
}
impl std::fmt::Display for PreferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Invalid => write!(f, "Invalid or oversized Run preference file; preserved"),
            Self::Conflict => write!(
                f,
                "Run preferences changed externally; restart/reload before saving"
            ),
        }
    }
}
impl std::error::Error for PreferenceError {}
impl From<std::io::Error> for PreferenceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
pub enum Event {
    Loaded(BTreeMap<String, String>),
    Failed(PreferenceError),
}
enum Request {
    Save { root: String, id: String },
    Flush(mpsc::Sender<()>),
}
pub struct Preferences {
    pub events: mpsc::Receiver<Event>,
    sender: mpsc::Sender<Request>,
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, PreferenceError> {
    match std::fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(65537).read_to_end(&mut bytes)?;
            if bytes.len() > 65536 {
                return Err(PreferenceError::Invalid);
            }
            Ok(Some(bytes))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
impl Preferences {
    pub fn start(path: PathBuf) -> Self {
        let (sender, rx) = mpsc::channel();
        let (tx, events) = mpsc::channel();
        std::thread::spawn(move || {
            let initial = read(&path);
            let mut previous = None;
            let mut values = BTreeMap::<String, String>::new();
            let mut valid = false;
            match initial {
                Ok(bytes) => {
                    previous = bytes;
                    let parsed = previous
                        .as_ref()
                        .map(|b| serde_json::from_slice::<BTreeMap<String, String>>(b))
                        .transpose();
                    match parsed {
                        Ok(map) => {
                            values = map.unwrap_or_default();
                            valid = true;
                            let _ = tx.send(Event::Loaded(values.clone()));
                        }
                        Err(_) => {
                            let _ = tx.send(Event::Failed(PreferenceError::Invalid));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Event::Failed(e));
                }
            }
            while let Ok(request) = rx.recv() {
                let (root, id) = match request {
                    Request::Save { root, id } => (root, id),
                    Request::Flush(ack) => {
                        let _ = ack.send(());
                        continue;
                    }
                };
                let result = (|| {
                    if !valid || read(&path)? != previous {
                        return Err(PreferenceError::Conflict);
                    }
                    let mut next = values.clone();
                    next.insert(root, id);
                    let bytes =
                        serde_json::to_vec_pretty(&next).map_err(|_| PreferenceError::Invalid)?;
                    if bytes.len() > 65536 {
                        return Err(PreferenceError::Invalid);
                    }
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&temporary)?;
                    let written = (|| {
                        file.write_all(&bytes)?;
                        file.sync_all()?;
                        drop(file);
                        std::fs::rename(&temporary, &path)
                    })();
                    if let Err(e) = written {
                        let _ = std::fs::remove_file(&temporary);
                        return Err(e.into());
                    }
                    previous = Some(bytes);
                    values = next;
                    Ok(values.clone())
                })();
                let _ = tx.send(match result {
                    Ok(values) => Event::Loaded(values),
                    Err(e) => Event::Failed(e),
                });
            }
        });
        Self { events, sender }
    }
    pub fn flush(&self) {
        let (tx, rx) = mpsc::channel();
        if self.sender.send(Request::Flush(tx)).is_ok() {
            let _ = rx.recv_timeout(std::time::Duration::from_secs(2));
        }
    }
    pub fn remember(&self, root: &Path, id: String) {
        let _ = self.sender.send(Request::Save {
            root: root.to_string_lossy().into_owned(),
            id,
        });
    }
}
pub(crate) fn manifest_text(path: &Path) -> std::io::Result<String> {
    let mut text = String::new();
    std::fs::File::open(path)?
        .take(1024 * 1024 + 1)
        .read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Manifest limit",
        ));
    }
    Ok(text)
}
pub(crate) fn rust_targets(project: &buture_project::ProjectMetadata) -> Vec<Task> {
    let mut names = std::collections::BTreeSet::new();
    if let Ok(text) = manifest_text(&project.manifest)
        && let Ok(doc) = toml::from_str::<toml::Value>(&text)
    {
        if let Some(bins) = doc.get("bin").and_then(toml::Value::as_array) {
            for bin in bins.iter().take(256) {
                if let Some(name) = bin.get("name").and_then(toml::Value::as_str) {
                    names.insert(name.to_owned());
                }
            }
        }
        if project.root.join("src/main.rs").is_file() {
            names.insert(project.name.clone());
        }
    }
    if let Ok(entries) = std::fs::read_dir(project.root.join("src/bin")) {
        for entry in entries.flatten().take(256) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                names.insert(stem.into());
            } else if path.is_dir()
                && path.join("main.rs").is_file()
                && let Some(name) = path.file_name().and_then(|s| s.to_str())
            {
                names.insert(name.into());
            }
        }
    }
    names
        .into_iter()
        .filter(|name| name.len() <= 256)
        .map(|name| {
            super::task(
                &project.root,
                "cargo",
                &["run", "--bin", &name],
                &format!("{} · Run {name}", project.name),
                true,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_preserve_external_owner_edits() {
        let root =
            std::env::temp_dir().join(format!("buture-run-preferences-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture");
        let file = root.join("run.json");
        let store = Preferences::start(file.clone());
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Loaded(_))
        ));
        store.remember(&root, "target-one".into());
        store.flush();
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Loaded(_))
        ));
        std::fs::write(&file, "owner edit").expect("external");
        store.remember(&root, "target-two".into());
        store.flush();
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Failed(PreferenceError::Conflict))
        ));
        assert_eq!(std::fs::read_to_string(&file).expect("file"), "owner edit");
        drop(store);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn ambiguity_requires_choice_and_stale_preference_is_ignored() {
        let mut a = super::super::task(
            &std::env::temp_dir(),
            "cargo",
            &["run", "--bin", "a"],
            "a",
            true,
        );
        let b = super::super::task(
            &std::env::temp_dir(),
            "cargo",
            &["run", "--bin", "b"],
            "b",
            true,
        );
        let both = vec![a.clone(), b.clone()];
        assert!(matches!(choose(&both, None), Choice::Ambiguous(_)));
        assert_eq!(choose(&both, Some(&b.id)), Choice::Selected(b.id.clone()));
        assert!(matches!(
            choose(&both, Some("removed")),
            Choice::Ambiguous(_)
        ));
        a.run = false;
        assert_eq!(choose(&[a], None), Choice::Unavailable);
    }
}
