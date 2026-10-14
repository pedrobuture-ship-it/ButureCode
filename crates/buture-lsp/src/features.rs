//! Bounded feature responses and reviewed editor-only workspace edits.
use crate::{character, file_uri, uri_path};
use buture_editor::{Buffer, Edit, TextSnapshot};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Completion,
    Hover,
    Definition,
    References,
    Rename,
    Signature,
    DocumentSymbols,
    WorkspaceSymbols,
    CodeActions,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Completion => "Completion",
            Self::Hover => "Hover",
            Self::Definition => "Definition",
            Self::References => "References",
            Self::Rename => "Rename",
            Self::Signature => "Signature",
            Self::DocumentSymbols => "Document Symbols",
            Self::WorkspaceSymbols => "Workspace Symbols",
            Self::CodeActions => "Code Actions",
        }
    }
    pub fn method(self) -> &'static str {
        match self {
            Self::Completion => "textDocument/completion",
            Self::Hover => "textDocument/hover",
            Self::Definition => "textDocument/definition",
            Self::References => "textDocument/references",
            Self::Rename => "textDocument/rename",
            Self::Signature => "textDocument/signatureHelp",
            Self::DocumentSymbols => "textDocument/documentSymbol",
            Self::WorkspaceSymbols => "workspace/symbol",
            Self::CodeActions => "textDocument/codeAction",
        }
    }
    pub fn from_method(method: &str) -> Option<Self> {
        [
            Self::Completion,
            Self::Hover,
            Self::Definition,
            Self::References,
            Self::Rename,
            Self::Signature,
            Self::DocumentSymbols,
            Self::WorkspaceSymbols,
            Self::CodeActions,
        ]
        .into_iter()
        .find(|kind| kind.method() == method)
    }
    pub fn capability(self) -> &'static str {
        match self {
            Self::Completion => "completionProvider",
            Self::Hover => "hoverProvider",
            Self::Definition => "definitionProvider",
            Self::References => "referencesProvider",
            Self::Rename => "renameProvider",
            Self::Signature => "signatureHelpProvider",
            Self::DocumentSymbols => "documentSymbolProvider",
            Self::WorkspaceSymbols => "workspaceSymbolProvider",
            Self::CodeActions => "codeActionProvider",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Range {
    pub start: Value,
    pub end: Value,
}
#[derive(Debug, Clone)]
pub struct Location {
    pub path: PathBuf,
    pub range: Option<Range>,
}
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub text: String,
}
#[derive(Debug, Clone, Default)]
pub struct WorkspaceEdit {
    pub files: BTreeMap<PathBuf, Vec<TextEdit>>,
    pub versions: BTreeMap<PathBuf, i64>,
}
#[derive(Debug, Clone)]
pub struct Completion {
    pub text: String,
    pub edit: Option<TextEdit>,
    pub additional: Vec<TextEdit>,
}
#[derive(Debug, Clone)]
pub struct Entry {
    pub label: String,
    pub detail: String,
    pub location: Option<Location>,
    pub completion: Option<Completion>,
    pub edit: Option<WorkspaceEdit>,
    pub disabled: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct Result {
    pub entries: Vec<Entry>,
    pub text: String,
    pub edit: Option<WorkspaceEdit>,
}
#[derive(Debug, Clone)]
pub struct Marker {
    pub severity: u8,
    pub message: String,
}
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub range: Range,
    pub severity: u8,
    pub message: String,
    pub source: String,
}
#[derive(Debug)]
pub enum FeatureError {
    InvalidResult,
    UnsupportedResourceOperation,
    OutsideWorkspace,
    TooLarge,
    Io(std::io::Error),
    Editor(buture_editor::EditorError),
}
impl std::fmt::Display for FeatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidResult => write!(f, "Invalid language-service result or UTF-16 range"),
            Self::UnsupportedResourceOperation => write!(
                f,
                "Create/delete/rename resource operations require a separate workflow"
            ),
            Self::OutsideWorkspace => write!(f, "Workspace edit is outside the trusted root"),
            Self::TooLarge => write!(
                f,
                "Language edit limit: 64 files, 2 MiB per file, 32 MiB total"
            ),
            Self::Io(e) => write!(f, "{e}"),
            Self::Editor(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for FeatureError {}
impl From<std::io::Error> for FeatureError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<buture_editor::EditorError> for FeatureError {
    fn from(e: buture_editor::EditorError) -> Self {
        Self::Editor(e)
    }
}
fn text(value: &Value, limit: usize) -> String {
    value.as_str().unwrap_or("").chars().take(limit).collect()
}
fn range(value: &Value) -> Option<Range> {
    let start = value.get("start")?;
    let end = value.get("end")?;
    for position in [start, end] {
        if position["line"].as_u64()? > i32::MAX as u64
            || position["character"].as_u64()? > i32::MAX as u64
        {
            return None;
        }
    }
    Some(Range {
        start: start.clone(),
        end: end.clone(),
    })
}
fn edit(value: &Value) -> std::result::Result<TextEdit, FeatureError> {
    let new = value["newText"]
        .as_str()
        .ok_or(FeatureError::InvalidResult)?;
    if new.len() > 2 * 1024 * 1024 {
        return Err(FeatureError::TooLarge);
    }
    Ok(TextEdit {
        range: range(&value["range"])
            .or_else(|| range(&value["replace"]))
            .ok_or(FeatureError::InvalidResult)?,
        text: new.into(),
    })
}
fn location(value: &Value) -> Option<Location> {
    let uri = value["uri"]
        .as_str()
        .or_else(|| value["targetUri"].as_str())?;
    Some(Location {
        path: uri_path(uri)?,
        range: range(&value["range"])
            .or_else(|| range(&value["targetSelectionRange"]))
            .or_else(|| range(&value["targetRange"])),
    })
}
fn entry(label: String) -> Entry {
    Entry {
        label,
        detail: String::new(),
        location: None,
        completion: None,
        edit: None,
        disabled: None,
    }
}
fn markup(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        s.chars().take(16384).collect()
    } else if value.is_array() {
        value
            .as_array()
            .into_iter()
            .flatten()
            .take(16)
            .map(markup)
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .take(16384)
            .collect()
    } else {
        text(&value["value"], 16384)
    }
}
pub fn workspace_edit(value: &Value) -> std::result::Result<WorkspaceEdit, FeatureError> {
    let mut result = WorkspaceEdit::default();
    let mut count = 0usize;
    if let Some(changes) = value["changes"].as_object() {
        for (uri, edits) in changes {
            let path = uri_path(uri).ok_or(FeatureError::OutsideWorkspace)?;
            let edits = edits.as_array().ok_or(FeatureError::InvalidResult)?;
            count += edits.len();
            if count > 4096 || result.files.len() >= 64 {
                return Err(FeatureError::TooLarge);
            }
            let parsed = edits
                .iter()
                .map(edit)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            result.files.entry(path).or_default().extend(parsed);
        }
    }
    if let Some(changes) = value["documentChanges"].as_array() {
        for change in changes {
            if change.get("kind").is_some() {
                return Err(FeatureError::UnsupportedResourceOperation);
            }
            let path = uri_path(
                change["textDocument"]["uri"]
                    .as_str()
                    .ok_or(FeatureError::InvalidResult)?,
            )
            .ok_or(FeatureError::OutsideWorkspace)?;
            if let Some(version) = change["textDocument"]["version"].as_i64() {
                result.versions.insert(path.clone(), version);
            }
            let edits = change["edits"]
                .as_array()
                .ok_or(FeatureError::InvalidResult)?;
            count += edits.len();
            if count > 4096 || result.files.len() >= 64 {
                return Err(FeatureError::TooLarge);
            }
            result.files.entry(path).or_default().extend(
                edits
                    .iter()
                    .map(edit)
                    .collect::<std::result::Result<Vec<_>, _>>()?,
            );
        }
    }
    Ok(result)
}
pub fn parse(
    kind: Kind,
    value: &Value,
    context: Option<&TextSnapshot>,
) -> std::result::Result<Result, FeatureError> {
    let mut result = Result::default();
    match kind {
        Kind::Hover => result.text = markup(&value["contents"]),
        Kind::Signature => {
            if let Some(signatures) = value["signatures"].as_array() {
                result.text = signatures
                    .iter()
                    .take(32)
                    .map(|s| {
                        format!(
                            "{}\n{}",
                            text(&s["label"], 4096),
                            markup(&s["documentation"])
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    .chars()
                    .take(16384)
                    .collect();
            }
        }
        Kind::Completion => {
            let items = value.as_array().or_else(|| value["items"].as_array());
            if let Some(items) = items {
                for item in items.iter().take(1000) {
                    let mut e = entry(text(&item["label"], 512));
                    e.detail = text(&item["detail"], 1024);
                    if item["insertTextFormat"].as_u64() == Some(2) {
                        e.disabled = Some("Snippet completion was not negotiated".into());
                    } else {
                        let primary = item.get("textEdit").map(edit).transpose()?;
                        if item["additionalTextEdits"]
                            .as_array()
                            .is_some_and(|items| items.len() > 128)
                        {
                            return Err(FeatureError::TooLarge);
                        }
                        let additional = item["additionalTextEdits"]
                            .as_array()
                            .map(|items| {
                                items
                                    .iter()
                                    .take(128)
                                    .map(edit)
                                    .collect::<std::result::Result<Vec<_>, _>>()
                            })
                            .transpose()?
                            .unwrap_or_default();
                        let insert = item["insertText"]
                            .as_str()
                            .or_else(|| item["label"].as_str())
                            .unwrap_or("");
                        if insert.len() > 65536 {
                            return Err(FeatureError::TooLarge);
                        }
                        e.completion = Some(Completion {
                            text: insert.to_owned(),
                            edit: primary,
                            additional,
                        });
                    }
                    result.entries.push(e);
                }
            }
        }
        Kind::Definition | Kind::References => {
            let locations = if let Some(array) = value.as_array() {
                array.iter().collect::<Vec<_>>()
            } else if value.is_object() {
                vec![value]
            } else {
                vec![]
            };
            for loc in locations.into_iter().take(1000) {
                if let Some(location) = location(loc) {
                    let mut e = entry(
                        location
                            .path
                            .file_name()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    );
                    e.detail = location.path.display().to_string();
                    e.location = Some(location);
                    result.entries.push(e);
                }
            }
        }
        Kind::Rename => {
            if !value.is_null() {
                result.edit = Some(workspace_edit(value)?);
            }
        }
        Kind::CodeActions => {
            if let Some(actions) = value.as_array() {
                for action in actions.iter().take(1000) {
                    let mut e = entry(text(&action["title"], 512));
                    if let Some(edit) = action.get("edit") {
                        e.edit = Some(workspace_edit(edit)?);
                    }
                    if let Some(reason) = action["disabled"]["reason"].as_str() {
                        e.disabled = Some(reason.chars().take(1024).collect());
                    } else if e.edit.is_none() {
                        e.disabled=Some("Server commands need a separate execution review; no text edit supplied".into());
                    }
                    result.entries.push(e);
                }
            }
        }
        Kind::DocumentSymbols | Kind::WorkspaceSymbols => {
            fn symbols(
                values: &Value,
                context: Option<&TextSnapshot>,
                depth: usize,
                out: &mut Vec<Entry>,
            ) {
                if depth >= 32 || out.len() >= 1000 {
                    return;
                }
                if let Some(array) = values.as_array() {
                    for symbol in array.iter().take(1000) {
                        if out.len() >= 1000 {
                            break;
                        }
                        let mut e = entry(format!(
                            "{}{}",
                            "  ".repeat(depth),
                            text(&symbol["name"], 512)
                        ));
                        e.detail = text(&symbol["detail"], 1024);
                        e.location = location(&symbol["location"]).or_else(|| {
                            context.and_then(|s| {
                                range(&symbol["selectionRange"])
                                    .or_else(|| range(&symbol["range"]))
                                    .map(|range| Location {
                                        path: s.path.clone(),
                                        range: Some(range),
                                    })
                            })
                        });
                        out.push(e);
                        symbols(&symbol["children"], context, depth + 1, out);
                    }
                }
            }
            symbols(value, context, 0, &mut result.entries);
        }
    }
    Ok(result)
}
pub fn markers(items: &[Diagnostic]) -> std::sync::Arc<BTreeMap<usize, Marker>> {
    let mut map = BTreeMap::<usize, Marker>::new();
    for item in items {
        let line = item.range.start["line"]
            .as_u64()
            .and_then(|line| usize::try_from(line).ok());
        if let Some(line) = line
            && map.get(&line).is_none_or(|m| item.severity < m.severity)
        {
            map.insert(
                line,
                Marker {
                    severity: item.severity,
                    message: item.message.clone(),
                },
            );
        }
    }
    std::sync::Arc::new(map)
}
pub fn diagnostics(params: &Value) -> Vec<Diagnostic> {
    let Some(path) = params["uri"].as_str().and_then(uri_path) else {
        return vec![];
    };
    params["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .take(5000)
        .filter_map(|value| {
            Some(Diagnostic {
                path: path.clone(),
                range: range(&value["range"])?,
                severity: value["severity"].as_u64().unwrap_or(3).clamp(1, 4) as u8,
                message: text(&value["message"], 4096),
                source: if value["source"].is_string() {
                    text(&value["source"], 256)
                } else {
                    "Language server".into()
                },
            })
        })
        .collect()
}
pub fn buffer_edits(
    snapshot: &TextSnapshot,
    edits: &[TextEdit],
) -> std::result::Result<Vec<Edit>, FeatureError> {
    edits
        .iter()
        .map(|edit| {
            let start =
                character(snapshot, &edit.range.start).ok_or(FeatureError::InvalidResult)?;
            let end = character(snapshot, &edit.range.end).ok_or(FeatureError::InvalidResult)?;
            if start > end {
                return Err(FeatureError::InvalidResult);
            }
            Ok(Edit {
                range: start..end,
                text: edit.text.clone(),
            })
        })
        .collect()
}

#[derive(Debug)]
pub struct PreparedFile {
    pub expected: TextSnapshot,
    pub buffer: Buffer,
    pub summary: String,
}
#[derive(Debug)]
pub struct PreparedEdit {
    pub files: Vec<PreparedFile>,
}
pub fn prepare(
    edit: WorkspaceEdit,
    root: &Path,
    open: &[Buffer],
) -> std::result::Result<PreparedEdit, FeatureError> {
    let root = std::fs::canonicalize(root)?;
    let mut files = vec![];
    let mut total = 0;
    for (path, edits) in edit.files {
        if !path.is_absolute() || std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(FeatureError::OutsideWorkspace);
        }
        let path = std::fs::canonicalize(path)?;
        if !path.starts_with(&root) {
            return Err(FeatureError::OutsideWorkspace);
        }
        let mut buffer = if let Some(buffer) = open.iter().find(|b| b.path.as_ref() == Some(&path))
        {
            buffer.clone()
        } else {
            let mut bytes = vec![];
            std::fs::File::open(&path)?
                .take(2 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 2 * 1024 * 1024 {
                return Err(FeatureError::TooLarge);
            }
            let text = String::from_utf8(bytes).map_err(|_| FeatureError::InvalidResult)?;
            let mut buffer = Buffer::from_text(&text);
            buffer.path = Some(path.clone());
            buffer
        };
        total += buffer.len_bytes();
        if total > 32 * 1024 * 1024 || buffer.len_bytes() > 2 * 1024 * 1024 {
            return Err(FeatureError::TooLarge);
        }
        let expected = buffer.snapshot();
        let edits = buffer_edits(&expected, &edits)?;
        let mut preview = String::new();
        for edit in edits.iter().take(4) {
            let before = expected
                .rope
                .slice(edit.range.clone())
                .chars()
                .take(160)
                .collect::<String>();
            let after = edit.text.chars().take(160).collect::<String>();
            preview.push_str(&format!("- {before}\n+ {after}\n"));
        }
        buffer.apply_edits(&edits)?;
        total += buffer.len_bytes().saturating_sub(expected.rope.len_bytes());
        if total > 32 * 1024 * 1024 || buffer.len_bytes() > 2 * 1024 * 1024 {
            return Err(FeatureError::TooLarge);
        }
        files.push(PreparedFile {
            expected,
            buffer,
            summary: preview,
        });
    }
    Ok(PreparedEdit { files })
}
pub fn version_matches(edit: &WorkspaceEdit, versions: &BTreeMap<String, i32>) -> bool {
    edit.versions.iter().all(|(path, version)| {
        versions
            .get(&file_uri(path))
            .is_some_and(|current| i64::from(*current) == *version)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_and_resource_edits_are_validated() {
        let r=parse(Kind::Completion,&serde_json::json!({"items":[{"label":"print","textEdit":{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":2}},"newText":"print"}}]}),None).expect("parse");
        let buffer = Buffer::from_text("pr🦀");
        let edits = buffer_edits(
            &buffer.snapshot(),
            &[r.entries[0]
                .completion
                .as_ref()
                .expect("completion")
                .edit
                .clone()
                .expect("edit")],
        )
        .expect("range");
        assert_eq!(edits[0].range, 0..2);
        assert!(matches!(
            workspace_edit(
                &serde_json::json!({"documentChanges":[{"kind":"delete","uri":"file:///private"}]})
            ),
            Err(FeatureError::UnsupportedResourceOperation)
        ));
    }
    #[test]
    fn prepared_edit_changes_buffers_only_and_keeps_undo() {
        let root = std::env::temp_dir().join(format!("buture-lsp-edit-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture");
        let path = root.join("a.rs");
        std::fs::write(&path, "let café = 1;\n").expect("file");
        let value = serde_json::json!({"changes":{file_uri(&path):[{"range":{"start":{"line":0,"character":4},"end":{"line":0,"character":8}},"newText":"tea"}]}});
        let mut prepared =
            prepare(workspace_edit(&value).expect("edit"), &root, &[]).expect("prepare");
        assert_eq!(
            std::fs::read_to_string(&path).expect("disk"),
            "let café = 1;\n"
        );
        assert_eq!(
            prepared.files[0].buffer.rope().to_string(),
            "let tea = 1;\n"
        );
        prepared.files[0].buffer.undo();
        assert_eq!(
            prepared.files[0].buffer.rope().to_string(),
            "let café = 1;\n"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
