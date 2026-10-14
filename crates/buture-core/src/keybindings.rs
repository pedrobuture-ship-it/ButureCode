//! Platform defaults, validated user overrides, conflict detection and two-stroke chords.
use crate::commands::{CommandRegistry, Scope};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Windows,
    Linux,
}
impl Platform {
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOS
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stroke {
    pub key: String,
    pub ctrl: bool,
    pub command: bool,
    pub alt: bool,
    pub shift: bool,
}
impl Stroke {
    pub fn label(&self, platform: Platform) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".into());
        }
        if self.command {
            parts.push(
                if platform == Platform::MacOS {
                    "Cmd"
                } else {
                    "Super"
                }
                .into(),
            );
        }
        if self.alt {
            parts.push(
                if platform == Platform::MacOS {
                    "Option"
                } else {
                    "Alt"
                }
                .into(),
            );
        }
        if self.shift {
            parts.push("Shift".into());
        }
        parts.push(self.key.clone());
        parts.join("+")
    }
    fn parse(text: &str, platform: Platform, line: usize) -> Result<Self, KeymapError> {
        let parts: Vec<_> = text.split('+').collect();
        if parts.is_empty() || parts.iter().any(|s| s.is_empty()) {
            return Err(KeymapError::InvalidStroke(line));
        }
        let mut stroke = Self {
            key: String::new(),
            ctrl: false,
            command: false,
            alt: false,
            shift: false,
        };
        for modifier in &parts[..parts.len() - 1] {
            match modifier.to_ascii_lowercase().as_str() {
                "primary" => {
                    if platform == Platform::MacOS {
                        stroke.command = true
                    } else {
                        stroke.ctrl = true
                    }
                }
                "ctrl" | "control" => stroke.ctrl = true,
                "cmd" | "command" | "super" if platform == Platform::MacOS => stroke.command = true,
                "alt" | "option" => stroke.alt = true,
                "shift" => stroke.shift = true,
                _ => return Err(KeymapError::InvalidStroke(line)),
            }
        }
        let key = parts.last().ok_or(KeymapError::InvalidStroke(line))?;
        stroke.key = canonical_key(key).ok_or(KeymapError::InvalidStroke(line))?;
        Ok(stroke)
    }
}
fn canonical_key(key: &str) -> Option<String> {
    if key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic() {
        return Some(key.to_ascii_uppercase());
    }
    if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() {
        return Some(format!("Num{key}"));
    }
    let named = match key.to_ascii_lowercase().as_str() {
        "enter" => "Enter",
        "escape" | "esc" => "Escape",
        "tab" => "Tab",
        "space" => "Space",
        "backspace" => "Backspace",
        "delete" => "Delete",
        "home" => "Home",
        "end" => "End",
        "arrowleft" | "left" => "ArrowLeft",
        "arrowright" | "right" => "ArrowRight",
        "arrowup" | "up" => "ArrowUp",
        "arrowdown" | "down" => "ArrowDown",
        "pageup" => "PageUp",
        "pagedown" => "PageDown",
        "slash" | "/" => "Slash",
        "backslash" | "\\" => "Backslash",
        "openbracket" | "[" => "OpenBracket",
        "closebracket" | "]" => "CloseBracket",
        "comma" => "Comma",
        "period" => "Period",
        "minus" => "Minus",
        "plus" => "Plus",
        "backtick" => "Backtick",
        _ => "",
    };
    if !named.is_empty() {
        return Some(named.into());
    }
    if let Some(number) = key
        .strip_prefix(['F', 'f'])
        .and_then(|s| s.parse::<u8>().ok())
        .filter(|n| (1..=24).contains(n))
    {
        return Some(format!("F{number}"));
    }
    None
}
#[derive(Debug, Clone)]
pub struct Binding {
    pub command: String,
    pub scope: Scope,
    pub keys: Vec<Stroke>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapError {
    InvalidLine(usize),
    InvalidStroke(usize),
    UnknownCommand(usize),
    DuplicateOverride(usize),
    Conflict { first: String, second: String },
    TooLarge,
}
impl std::fmt::Display for KeymapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLine(l) => {
                write!(f, "Invalid keybinding line {l}; use command.id = shortcut")
            }
            Self::InvalidStroke(l) => write!(f, "Unsupported shortcut at line {l}"),
            Self::UnknownCommand(l) => write!(f, "Unknown command at line {l}"),
            Self::DuplicateOverride(l) => write!(f, "Duplicate command override at line {l}"),
            Self::Conflict { first, second } => write!(
                f,
                "Conflicting shortcut or chord prefix: {first} and {second}"
            ),
            Self::TooLarge => write!(f, "Keybinding profile exceeds its limit"),
        }
    }
}
impl std::error::Error for KeymapError {}
#[derive(Debug, Clone)]
pub struct Keymap {
    pub platform: Platform,
    pub bindings: Vec<Binding>,
    pending: Option<(Stroke, f64)>,
}
impl Keymap {
    pub fn defaults(platform: Platform, registry: &CommandRegistry) -> Self {
        let defaults = [
            ("workspace.open", "Primary+O"),
            ("file.open", "Primary+Shift+O"),
            ("files.quickOpen", "Primary+P"),
            ("commands.open", "Primary+Shift+P,Primary+K"),
            ("search.document", "Primary+F"),
            ("search.workspace", "Primary+Shift+F"),
            ("settings.open", "Primary+Comma"),
            ("effects.cycle", "Primary+Alt+E"),
            ("panel.toggle", "Primary+J"),
            ("inspector.toggle", "Primary+Alt+I"),
            ("editor.save", "Primary+S"),
            ("format.document", "Primary+Shift+I"),
            ("format.selection", "Primary+Alt+Shift+I"),
            ("language.completion", "Primary+Space"),
            ("language.definition", "F12"),
            ("language.references", "Shift+F12"),
            ("language.rename", "F2"),
            ("language.signature", "Primary+Shift+Space"),
            ("language.codeActions", "Primary+Period"),
            ("editor.close", "Primary+W"),
            ("editor.restore", "Primary+Shift+T"),
            ("editor.splitRight", "Primary+Backslash"),
            ("editor.splitDown", "Primary+Shift+Backslash"),
            ("editor.undo", "Primary+Z"),
            ("editor.redo", "Primary+Shift+Z"),
            ("editor.selectAll", "Primary+A"),
            ("editor.duplicate", "Alt+Shift+ArrowDown"),
            ("editor.moveUp", "Alt+ArrowUp"),
            ("editor.moveDown", "Alt+ArrowDown"),
            ("editor.comment", "Primary+Slash"),
            ("editor.indent", "Primary+CloseBracket"),
            ("editor.outdent", "Primary+OpenBracket,Shift+Tab"),
            ("editor.tab", "Tab"),
            ("editor.occurrence", "Primary+D"),
            ("editor.cursorUp", "Primary+Alt+ArrowUp"),
            ("editor.cursorDown", "Primary+Alt+ArrowDown"),
            ("editor.clearCursors", "Escape"),
            ("editor.fold", "Primary+Alt+F"),
        ];
        let mut bindings = Vec::new();
        for (id, keys) in defaults {
            if let Some(command) = registry.get(id) {
                for key in keys.split(',') {
                    if let Ok(key) = Stroke::parse(key, platform, 0) {
                        bindings.push(Binding {
                            command: id.into(),
                            scope: command.scope,
                            keys: vec![key],
                        });
                    }
                }
            }
        }
        if platform != Platform::MacOS
            && let Some(command) = registry.get("editor.redo")
            && let Ok(key) = Stroke::parse("Ctrl+Y", platform, 0)
        {
            bindings.push(Binding {
                command: command.id.into(),
                scope: command.scope,
                keys: vec![key],
            });
        }
        Self {
            platform,
            bindings,
            pending: None,
        }
    }
    pub fn overrides(
        text: &str,
        platform: Platform,
        registry: &CommandRegistry,
    ) -> Result<Self, KeymapError> {
        if text.len() > 64 * 1024 {
            return Err(KeymapError::TooLarge);
        }
        let mut keymap = Self::defaults(platform, registry);
        let mut seen = std::collections::BTreeSet::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line_number = index + 1;
            let (id, value) = line
                .split_once('=')
                .ok_or(KeymapError::InvalidLine(line_number))?;
            let id = id.trim();
            let command = registry
                .get(id)
                .ok_or(KeymapError::UnknownCommand(line_number))?;
            if !seen.insert(id) {
                return Err(KeymapError::DuplicateOverride(line_number));
            }
            keymap.bindings.retain(|b| b.command != id);
            let value = value.trim();
            if value.eq_ignore_ascii_case("none") {
                continue;
            }
            let aliases: Vec<_> = value.split(',').collect();
            if aliases.len() > 4 {
                return Err(KeymapError::TooLarge);
            }
            for alias in aliases {
                let keys = alias
                    .split_whitespace()
                    .map(|s| Stroke::parse(s, platform, line_number))
                    .collect::<Result<Vec<_>, _>>()?;
                if keys.is_empty() || keys.len() > 2 {
                    return Err(KeymapError::InvalidStroke(line_number));
                }
                keymap.bindings.push(Binding {
                    command: id.into(),
                    scope: command.scope,
                    keys,
                });
            }
        }
        for (index, a) in keymap.bindings.iter().enumerate() {
            for b in keymap.bindings.iter().skip(index + 1) {
                let scopes =
                    a.scope == b.scope || a.scope == Scope::Global || b.scope == Scope::Global;
                let prefix = a.keys.starts_with(&b.keys) || b.keys.starts_with(&a.keys);
                if scopes && prefix && (a.command != b.command || a.keys.len() != b.keys.len()) {
                    return Err(KeymapError::Conflict {
                        first: a.command.clone(),
                        second: b.command.clone(),
                    });
                }
            }
        }
        Ok(keymap)
    }
    pub fn label(&self, command: &str) -> String {
        self.bindings
            .iter()
            .filter(|b| b.command == command)
            .map(|b| {
                b.keys
                    .iter()
                    .map(|k| k.label(self.platform))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
    pub fn tick(&mut self, time: f64) {
        if self
            .pending
            .as_ref()
            .is_some_and(|(_, started)| time - *started > 1.5)
        {
            self.pending = None;
        }
    }
    pub fn pending_label(&self) -> Option<String> {
        self.pending
            .as_ref()
            .map(|(key, _)| key.label(self.platform))
    }
    pub fn process(&mut self, key: Stroke, editor: bool, time: f64) -> Outcome {
        self.tick(time);
        let active = |binding: &Binding| binding.scope == Scope::Global || editor;
        if let Some((first, _)) = self.pending.take()
            && let Some(binding) = self
                .bindings
                .iter()
                .find(|b| active(b) && b.keys.len() == 2 && b.keys[0] == first && b.keys[1] == key)
        {
            return Outcome::Command(binding.command.clone());
        }
        if let Some(binding) = self.bindings.iter().find(|b| active(b) && b.keys[0] == key) {
            if binding.keys.len() == 1 {
                return Outcome::Command(binding.command.clone());
            }
            self.pending = Some((key, time));
            return Outcome::Pending;
        }
        Outcome::Ignored
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Ignored,
    Pending,
    Command(String),
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_follow_platform_and_editor_scope() {
        let registry = CommandRegistry::standard();
        let mac = Keymap::defaults(Platform::MacOS, &registry);
        assert_eq!(mac.label("editor.save"), "Cmd+S");
        let mut linux = Keymap::defaults(Platform::Linux, &registry);
        assert_eq!(linux.label("editor.save"), "Ctrl+S");
        let stroke = Stroke::parse("Primary+S", Platform::Linux, 1).expect("stroke");
        assert_eq!(linux.process(stroke.clone(), false, 0.0), Outcome::Ignored);
        assert_eq!(
            linux.process(stroke, true, 0.0),
            Outcome::Command("editor.save".into())
        );
    }
    #[test]
    fn overrides_conflicts_chords_and_timeouts() {
        let registry = CommandRegistry::standard();
        assert!(Keymap::overrides("editor.save=Primary+P", Platform::Linux, &registry).is_err());
        assert!(Keymap::overrides("shell.run=Primary+R", Platform::Linux, &registry).is_err());
        let mut map = Keymap::overrides(
            "commands.open=Primary+Shift+P\neditor.save=Primary+K Primary+S",
            Platform::Linux,
            &registry,
        )
        .expect("chord");
        let first = Stroke::parse("Primary+K", Platform::Linux, 1).expect("first");
        let second = Stroke::parse("Primary+S", Platform::Linux, 1).expect("second");
        assert_eq!(map.process(first.clone(), true, 0.0), Outcome::Pending);
        assert_eq!(
            map.process(second.clone(), true, 0.5),
            Outcome::Command("editor.save".into())
        );
        assert_eq!(map.process(first, true, 2.0), Outcome::Pending);
        assert_eq!(map.process(second, true, 4.0), Outcome::Ignored);
        let disabled =
            Keymap::overrides("editor.save=none", Platform::Linux, &registry).expect("disable");
        assert!(disabled.label("editor.save").is_empty());
    }
}
