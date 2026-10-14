//! Layered preferences, validated INI persistence and background IO.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VisualEffects {
    #[default]
    Full,
    Reduced,
    Off,
}

impl VisualEffects {
    pub const ALL: [Self; 3] = [Self::Full, Self::Reduced, Self::Off];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Reduced => "Reduced",
            Self::Off => "Off",
        }
    }
}

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};
pub const CATEGORIES: [&str; 15] = [
    "Appearance",
    "Editor",
    "Files",
    "Terminal",
    "Languages",
    "Toolchains",
    "Git",
    "Debug",
    "Database",
    "Docker",
    "Hardware",
    "Remote",
    "AI",
    "Privacy",
    "Development",
];
#[derive(Debug)]
pub enum SettingsError {
    Io(io::Error),
    Invalid { line: usize },
    UnknownKey(String),
    InvalidValue(String),
}
impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Invalid { line } => write!(f, "Invalid settings at line {line}"),
            Self::UnknownKey(k) => write!(f, "Unsupported setting: {k}"),
            Self::InvalidValue(k) => write!(f, "Invalid value for {k}"),
        }
    }
}
impl std::error::Error for SettingsError {}
impl From<io::Error> for SettingsError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
#[derive(Debug, Clone, Default)]
pub struct Layer {
    values: BTreeMap<String, String>,
}
const SCHEMA: [(&str, &str); 9] = [
    ("Appearance.effects", "Full"),
    ("Editor.formatOnSave", "false"),
    ("Appearance.inspector", "true"),
    ("Appearance.panel", "true"),
    ("Files.previewLimitKiB", "1024"),
    ("Languages.selected", ""),
    ("Development.onboardingComplete", "false"),
    ("Privacy.telemetry", "false"),
    ("AI.enabled", "false"),
];
impl Layer {
    pub fn parse(text: &str) -> Result<Self, SettingsError> {
        let mut layer = Self::default();
        let mut section = "";
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(s) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if !CATEGORIES.contains(&s) {
                    return Err(SettingsError::Invalid { line: index + 1 });
                }
                section = s;
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or(SettingsError::Invalid { line: index + 1 })?;
            let key = format!("{section}.{}", key.trim());
            if layer.values.contains_key(&key) {
                return Err(SettingsError::Invalid { line: index + 1 });
            }
            layer.set(&key, value.trim())?;
        }
        Ok(layer)
    }
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), SettingsError> {
        if !SCHEMA.iter().any(|(k, _)| *k == key) {
            return Err(SettingsError::UnknownKey(key.into()));
        }
        let valid = match key {
            "Appearance.effects" => ["Full", "Reduced", "Off"].contains(&value),
            "Languages.selected" => {
                value.is_empty()
                    || value.split(',').all(|v| {
                        [
                            "Rust",
                            "C",
                            "C++",
                            "Python",
                            "Java",
                            "Node.js",
                            "TypeScript",
                            "Go",
                            "C#",
                            "Kotlin",
                            "PHP",
                            "Ruby",
                        ]
                        .contains(&v)
                    })
            }
            "Files.previewLimitKiB" => value == "1024",
            "Privacy.telemetry" | "AI.enabled" => value == "false",
            _ => ["true", "false"].contains(&value),
        };
        if !valid {
            return Err(SettingsError::InvalidValue(key.into()));
        }
        self.values.insert(key.into(), value.into());
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, SettingsError> {
        match fs::read_to_string(path) {
            Ok(s) => Self::parse(&s),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut text = String::from("# Buture Code preferences. Secrets are not supported.\n");
        for category in CATEGORIES {
            text.push_str(&format!("\n[{category}]\n"));
            for (key, value) in &self.values {
                if let Some(k) = key.strip_prefix(&format!("{category}.")) {
                    text.push_str(&format!("{k} = {value}\n"));
                }
            }
        }
        // A new temporary file prevents following a pre-existing temporary symlink.
        let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    }
}
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub user: Layer,
    pub workspace: Layer,
}
impl Settings {
    pub fn get(&self, key: &str) -> Option<&str> {
        // Workspace content cannot complete onboarding or opt into private features.
        let local = if key == "Editor.formatOnSave"
            || key.starts_with("Development.")
            || key.starts_with("Privacy.")
            || key.starts_with("AI.")
        {
            None
        } else {
            self.workspace.values.get(key)
        };
        local
            .or_else(|| self.user.values.get(key))
            .map(String::as_str)
            .or_else(|| SCHEMA.iter().find(|(k, _)| *k == key).map(|(_, v)| *v))
    }
    pub fn effects(&self) -> VisualEffects {
        match self.get("Appearance.effects") {
            Some("Off") => VisualEffects::Off,
            Some("Reduced") => VisualEffects::Reduced,
            _ => VisualEffects::Full,
        }
    }
}
pub fn user_path() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }
    .map(|p| p.join("buture-code/settings.ini"))
}
pub enum Request {
    LoadWorkspace(PathBuf),
    SetUser { key: String, value: String },
}
pub struct Preferences {
    pub requests: std::sync::mpsc::Sender<Request>,
    pub events: std::sync::mpsc::Receiver<Result<Settings, SettingsError>>,
}
impl Preferences {
    pub fn start() -> Self {
        let (requests, rx) = std::sync::mpsc::channel();
        let (tx, events) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let path = user_path();
            let mut settings = Settings::default();
            let mut loaded = false;
            match path.as_ref().map(|p| Layer::load(p)).transpose() {
                Ok(layer) => {
                    settings.user = layer.unwrap_or_default();
                    loaded = true;
                    let _ = tx.send(Ok(settings.clone()));
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                }
            }
            while let Ok(request) = rx.recv() {
                let result = (|| {
                    match request {
                        Request::LoadWorkspace(root) => {
                            settings.workspace = Layer::load(&root.join(".buture/settings.ini"))?
                        }
                        Request::SetUser { key, value } => {
                            if !loaded {
                                return Err(SettingsError::Io(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "Existing user settings could not be read; refusing to overwrite them",
                                )));
                            }
                            let mut layer = settings.user.clone();
                            layer.set(&key, &value)?;
                            layer.save(path.as_ref().ok_or_else(|| {
                                SettingsError::Io(io::Error::new(
                                    io::ErrorKind::NotFound,
                                    "User configuration directory unavailable",
                                ))
                            })?)?;
                            settings.user = layer;
                        }
                    }
                    Ok(settings.clone())
                })();
                let _ = tx.send(result);
            }
        });
        Self { requests, events }
    }
}
pub mod keybindings;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_cannot_enable_format_on_save() {
        let mut settings = Settings::default();
        settings
            .workspace
            .set("Editor.formatOnSave", "true")
            .expect("fixture");
        assert_eq!(settings.get("Editor.formatOnSave"), Some("false"));
        settings
            .user
            .set("Editor.formatOnSave", "true")
            .expect("user choice");
        assert_eq!(settings.get("Editor.formatOnSave"), Some("true"));
    }
    #[test]
    fn layers_validate_and_preserve_privacy() {
        let user =
            Layer::parse("[Appearance]\neffects=Reduced\n[Development]\nonboardingComplete=true")
                .expect("user");
        let workspace =
            Layer::parse("[Appearance]\neffects=Off\n[Development]\nonboardingComplete=false")
                .expect("workspace");
        let settings = Settings { user, workspace };
        assert_eq!(settings.effects(), VisualEffects::Off);
        assert_eq!(settings.get("Development.onboardingComplete"), Some("true"));
        assert!(Layer::parse("[AI]\napiKey=secret").is_err());
        assert!(Layer::parse("[Privacy]\ntelemetry=true").is_err());
        assert!(Layer::parse("[Appearance]\neffects=Full\neffects=Off").is_err());
    }
    #[test]
    fn preferences_roundtrip_and_replace() {
        let root = std::env::temp_dir().join(format!("buture-settings-{}", std::process::id()));
        let path = root.join("settings.ini");
        let mut layer = Layer::parse("[Appearance]\neffects=Reduced").expect("parse");
        layer.save(&path).expect("save");
        layer.set("Appearance.effects", "Off").expect("set");
        layer.save(&path).expect("replace");
        assert_eq!(
            Settings {
                user: Layer::load(&path).expect("load"),
                workspace: Default::default()
            }
            .effects(),
            VisualEffects::Off
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
