//! Stable command identities and deterministic fuzzy matching, independent of a GUI or shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Editor,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenFile,
    OpenFolder,
    RunProject,
    Doctor,
    Settings,
    Effects,
    SearchDocument,
    SearchWorkspace,
    Palette,
    QuickOpen,
    TogglePanel,
    Terminal,
    ToggleInspector,
    Save,
    CloseFile,
    RestoreTab,
    SplitRight,
    SplitDown,
    Editor(&'static str),
    Unavailable(&'static str),
}
#[derive(Debug, Clone)]
pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    pub scope: Scope,
    pub action: Action,
}
#[derive(Default)]
pub struct CommandRegistry {
    pub commands: Vec<Command>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateId(String),
}
impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "Duplicate command ID: {id}"),
        }
    }
}
impl std::error::Error for RegistryError {}
impl CommandRegistry {
    pub fn register(&mut self, command: Command) -> Result<(), RegistryError> {
        if self.commands.iter().any(|c| c.id == command.id) {
            return Err(RegistryError::DuplicateId(command.id.into()));
        }
        self.commands.push(command);
        Ok(())
    }
    pub fn get(&self, id: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.id == id)
    }
    pub fn standard() -> Self {
        use Action::*;
        use Scope::{Editor as Editing, Global};
        let entries = [
            ("file.open", "Open File", Global, OpenFile),
            ("workspace.open", "Open Folder", Global, OpenFolder),
            ("project.run", "Run Project", Global, RunProject),
            ("doctor.scan", "Run Doctor", Global, Doctor),
            ("terminal.toggle", "Toggle Terminal", Global, Terminal),
            (
                "git.commit",
                "Git Commit",
                Global,
                Unavailable("Git commit integration is not available yet"),
            ),
            (
                "theme.change",
                "Change Theme",
                Global,
                Unavailable("Only Buture Glass Dark is available"),
            ),
            ("settings.open", "Settings", Global, Settings),
            ("effects.cycle", "Cycle Visual Effects", Global, Effects),
            (
                "search.document",
                "Find in Document",
                Editing,
                SearchDocument,
            ),
            (
                "search.workspace",
                "Search Workspace",
                Global,
                SearchWorkspace,
            ),
            ("commands.open", "Command Palette", Global, Palette),
            ("files.quickOpen", "Quick Open", Global, QuickOpen),
            ("panel.toggle", "Toggle Output Panel", Global, TogglePanel),
            (
                "inspector.toggle",
                "Toggle Inspector",
                Global,
                ToggleInspector,
            ),
            ("editor.save", "Save Document", Editing, Save),
            ("editor.close", "Close Tab", Editing, CloseFile),
            ("editor.restore", "Restore Closed Tab", Global, RestoreTab),
            (
                "editor.splitRight",
                "Split Editor Right",
                Editing,
                SplitRight,
            ),
            ("editor.splitDown", "Split Editor Down", Editing, SplitDown),
            ("editor.undo", "Undo", Editing, Editor("undo")),
            ("editor.redo", "Redo", Editing, Editor("redo")),
            (
                "editor.selectAll",
                "Select All",
                Editing,
                Editor("selectAll"),
            ),
            (
                "editor.duplicate",
                "Duplicate Line",
                Editing,
                Editor("duplicate"),
            ),
            ("editor.moveUp", "Move Lines Up", Editing, Editor("moveUp")),
            (
                "editor.moveDown",
                "Move Lines Down",
                Editing,
                Editor("moveDown"),
            ),
            (
                "editor.comment",
                "Comment / Uncomment",
                Editing,
                Editor("comment"),
            ),
            ("editor.indent", "Indent", Editing, Editor("indent")),
            ("editor.tab", "Insert Indentation", Editing, Editor("tab")),
            ("editor.outdent", "Outdent", Editing, Editor("outdent")),
            (
                "editor.occurrence",
                "Select Next Occurrence",
                Editing,
                Editor("occurrence"),
            ),
            (
                "editor.cursorUp",
                "Add Cursor Above",
                Editing,
                Editor("cursorUp"),
            ),
            (
                "editor.cursorDown",
                "Add Cursor Below",
                Editing,
                Editor("cursorDown"),
            ),
            (
                "editor.clearCursors",
                "Clear Extra Cursors",
                Editing,
                Editor("clearCursors"),
            ),
            ("editor.fold", "Toggle Fold", Editing, Editor("fold")),
        ];
        Self {
            commands: entries
                .into_iter()
                .map(|(id, label, scope, action)| Command {
                    id,
                    label,
                    scope,
                    action,
                })
                .collect(),
        }
    }
    pub fn search(&self, query: &str) -> Vec<usize> {
        let mut ranked: Vec<_> = self
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, c)| fuzzy_score(query, c.label).map(|score| (i, score)))
            .collect();
        ranked.sort_by(|(a, sa), (b, sb)| {
            sb.cmp(sa)
                .then_with(|| self.commands[*a].label.cmp(self.commands[*b].label))
        });
        ranked.into_iter().map(|(i, _)| i).collect()
    }
}
pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    let needle: Vec<_> = query.to_lowercase().chars().collect();
    let chars: Vec<_> = candidate.to_lowercase().chars().collect();
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > chars.len() {
        return None;
    }
    if needle == chars {
        return Some(10_000);
    }
    let mut score = 0;
    let mut position = 0;
    let mut previous = None;
    for c in needle {
        let offset = chars[position..].iter().position(|v| *v == c)?;
        let index = position + offset;
        score += 20 - offset as i64;
        if index == 0 || !chars[index - 1].is_alphanumeric() {
            score += 30;
        }
        if previous.is_some_and(|p| p + 1 == index) {
            score += 20;
        }
        previous = Some(index);
        position = index + 1;
    }
    Some(score - chars.len() as i64)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fuzzy_registry_keeps_ids_and_rejects_duplicate() {
        let mut registry = CommandRegistry::standard();
        assert_eq!(
            registry.commands[registry.search("run doctor")[0]].id,
            "doctor.scan"
        );
        assert!(fuzzy_score("rdo", "Run Doctor").is_some());
        assert!(fuzzy_score("zzzz", "Run Doctor").is_none());
        let command = registry.commands[0].clone();
        assert!(registry.register(command).is_err());
        assert!(matches!(
            registry.get("git.commit").expect("command").action,
            Action::Unavailable(_)
        ));
    }
}
