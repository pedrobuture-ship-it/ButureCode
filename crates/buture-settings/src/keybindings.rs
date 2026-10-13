//! User-only shortcut profile. Invalid or externally edited files are never silently overwritten.
use buture_core::{
    commands::CommandRegistry,
    keybindings::{Keymap, Platform},
};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
};
const LIMIT: u64 = 64 * 1024;
#[derive(Debug)]
pub enum ProfileError {
    Io(std::io::Error),
    Invalid(buture_core::keybindings::KeymapError),
    TooLarge,
    Encoding,
    Conflict,
}
impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Invalid(e) => write!(f, "{e}"),
            Self::TooLarge => write!(f, "Keybinding profile exceeds 64 KiB"),
            Self::Encoding => write!(f, "Keybinding profile must be UTF-8"),
            Self::Conflict => write!(
                f,
                "Keybinding profile changed externally; reload before saving"
            ),
        }
    }
}
impl std::error::Error for ProfileError {}
impl From<std::io::Error> for ProfileError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
pub enum Request {
    Reload,
    Save(String),
}
pub enum Event {
    Loaded {
        profile: String,
        keymap: Result<Keymap, ProfileError>,
    },
    Failed(ProfileError),
}
pub struct Store {
    pub events: mpsc::Receiver<Event>,
    sender: mpsc::Sender<Request>,
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, ProfileError> {
    match std::fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(LIMIT + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > LIMIT {
                return Err(ProfileError::TooLarge);
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
        std::thread::spawn(move || {
            let registry = CommandRegistry::standard();
            let platform = Platform::current();
            let mut previous = None;
            let mut loaded = false;
            let mut next = Request::Reload;
            loop {
                match next {
                    Request::Reload => {
                        loaded = false;
                        match read(&path) {
                            Ok(bytes) => {
                                previous = bytes;
                                match previous
                                    .as_ref()
                                    .map(|b| std::str::from_utf8(b).map(String::from))
                                    .transpose()
                                {
                                    Ok(profile) => {
                                        loaded = true;
                                        let profile=profile.unwrap_or_else(||"# Buture Code user shortcuts\n# command.id = Primary+Key\n# Use none to disable a command; comma separates aliases.\n".into());
                                        let keymap =
                                            Keymap::overrides(&profile, platform, &registry)
                                                .map_err(ProfileError::Invalid);
                                        let _ = tx.send(Event::Loaded { profile, keymap });
                                    }
                                    Err(_) => {
                                        let _ = tx.send(Event::Failed(ProfileError::Encoding));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(Event::Failed(e));
                            }
                        }
                    }
                    Request::Save(profile) => {
                        let result = (|| {
                            let keymap = Keymap::overrides(&profile, platform, &registry)
                                .map_err(ProfileError::Invalid)?;
                            if !loaded || read(&path)? != previous {
                                return Err(ProfileError::Conflict);
                            }
                            if let Some(parent) = path.parent() {
                                std::fs::create_dir_all(parent)?;
                            }
                            let temporary =
                                path.with_extension(format!("tmp-{}", std::process::id()));
                            let mut file = std::fs::OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(&temporary)?;
                            file.write_all(profile.as_bytes())?;
                            file.sync_all()?;
                            drop(file);
                            std::fs::rename(temporary, &path)?;
                            previous = Some(profile.as_bytes().to_vec());
                            Ok(keymap)
                        })();
                        let _ = tx.send(match result {
                            Ok(keymap) => Event::Loaded {
                                profile,
                                keymap: Ok(keymap),
                            },
                            Err(e) => Event::Failed(e),
                        });
                    }
                }
                match rx.recv() {
                    Ok(request) => next = request,
                    Err(_) => break,
                }
            }
        });
        Self { events, sender }
    }
    pub fn reload(&self) {
        let _ = self.sender.send(Request::Reload);
    }
    pub fn save(&self, profile: String) {
        let _ = self.sender.send(Request::Save(profile));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_and_external_profiles_are_preserved() {
        let root = std::env::temp_dir().join(format!("buture-key-profile-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture");
        let path = root.join("keys.ini");
        let store = Store::start(path.clone());
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Loaded { keymap: Ok(_), .. })
        ));
        store.save("editor.save=Primary+P".into());
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Failed(ProfileError::Invalid(_)))
        ));
        assert!(!path.exists());
        store.save("editor.save=Primary+Alt+S".into());
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Loaded { keymap: Ok(_), .. })
        ));
        std::fs::write(&path, "# owner edit").expect("external");
        store.save("editor.save=none".into());
        assert!(matches!(
            store.events.recv_timeout(std::time::Duration::from_secs(2)),
            Ok(Event::Failed(ProfileError::Conflict))
        ));
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "# owner edit"
        );
        drop(store);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
