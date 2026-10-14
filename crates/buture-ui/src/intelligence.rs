//! Language feature UI. Mutation is staged, stale-checked and applied only to editor buffers.
use crate::{Theme, editor::EditorSpace};
use buture_editor::{Buffer, TextSnapshot};
use buture_lsp::{
    Client,
    features::{self, Kind, Location, PreparedEdit, WorkspaceEdit},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
struct Query {
    kind: Kind,
    source: Option<TextSnapshot>,
    caret: usize,
    open: Vec<Buffer>,
    root: PathBuf,
}
#[derive(Default)]
pub struct Intelligence {
    pub visible: bool,
    kind: Option<Kind>,
    requested: bool,
    pending: Option<Query>,
    query: Option<Query>,
    result: Option<features::Result>,
    input: String,
    focus: bool,
    message: String,
    preparing: Option<mpsc::Receiver<std::result::Result<PreparedEdit, features::FeatureError>>>,
    prepared: Option<PreparedEdit>,
}
pub enum Action {
    Navigate(Location),
    Request(Kind),
}
impl Intelligence {
    pub fn open(&mut self, kind: Kind) {
        self.visible = true;
        self.kind = Some(kind);
        self.result = None;
        self.message.clear();
        self.requested = !matches!(kind, Kind::Rename | Kind::WorkspaceSymbols);
        self.input.clear();
        self.focus = true;
    }
    pub fn requested(&mut self) -> Option<Kind> {
        if self.requested {
            self.requested = false;
            self.kind
        } else {
            None
        }
    }
    pub fn request(
        &mut self,
        kind: Kind,
        client: &Client,
        capabilities: &Value,
        editors: &EditorSpace,
        root: &Path,
    ) {
        let cap = &capabilities[kind.capability()];
        if cap.is_null() || cap.as_bool() == Some(false) {
            self.message = "The server did not advertise this capability".into();
            return;
        }
        if self.pending.is_some() {
            self.message = "Wait for the pending language request".into();
            return;
        }
        let source = editors.active_snapshot();
        let caret = editors.caret_offset();
        let mut params = if kind == Kind::WorkspaceSymbols {
            json!({"query":self.input})
        } else {
            let Some(snapshot) = &source else {
                self.message = "Open a document first".into();
                return;
            };
            let Some(position) = buture_lsp::position(snapshot, caret) else {
                return;
            };
            json!({"textDocument":{"uri":buture_lsp::file_uri(&snapshot.path)},"position":position})
        };
        match kind {
            Kind::Rename => {
                if self.input.is_empty() {
                    self.message = "Enter the new name".into();
                    return;
                }
                params["newName"] = json!(self.input);
            }
            Kind::References => params["context"] = json!({"includeDeclaration":true}),
            Kind::CodeActions => {
                let Some(snapshot) = &source else {
                    return;
                };
                let range = editors.selection_range();
                params["range"] = json!({"start":buture_lsp::position(snapshot,range.start),"end":buture_lsp::position(snapshot,range.end)});
                params["context"] = json!({"diagnostics":[]});
            }
            _ => {}
        }
        let context = if kind == Kind::WorkspaceSymbols {
            None
        } else {
            source.clone()
        };
        match client.request(kind.method(), params, context) {
            Ok(()) => {
                self.pending = Some(Query {
                    kind,
                    source,
                    caret,
                    open: editors.open_buffers(),
                    root: root.into(),
                });
                self.message = "Waiting for language server…".into();
            }
            Err(e) => self.message = e.to_string(),
        }
    }
    pub fn error(&mut self, message: String) {
        self.message = message;
        self.pending = None;
    }
    pub fn receive(&mut self, kind: Kind, result: features::Result, context: Option<TextSnapshot>) {
        let Some(query) = self.pending.take().filter(|query| query.kind == kind) else {
            return;
        };
        if query
            .source
            .as_ref()
            .map(|s| (&s.path, s.revision, s.content_id))
            != context
                .as_ref()
                .map(|s| (&s.path, s.revision, s.content_id))
            && kind != Kind::WorkspaceSymbols
        {
            self.message = "Stale language result discarded".into();
            return;
        }
        self.query = Some(query);
        self.message =
            if result.entries.is_empty() && result.text.is_empty() && result.edit.is_none() {
                "No results returned by the server".into()
            } else {
                "Result from local language server".into()
            };
        self.result = Some(result);
    }
    fn prepare(&mut self, edit: WorkspaceEdit) {
        let Some(query) = &self.query else {
            return;
        };
        let root = query.root.clone();
        let open = query.open.clone();
        let (tx, rx) = mpsc::channel();
        self.preparing = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(features::prepare(edit, &root, &open));
        });
    }
    pub fn poll(&mut self) {
        if let Some(rx) = &self.preparing
            && let Ok(result) = rx.try_recv()
        {
            match result {
                Ok(prepared) => self.prepared = Some(prepared),
                Err(e) => self.message = e.to_string(),
            }
            self.preparing = None;
        }
    }
    pub fn review(&mut self, ctx: &egui::Context, editors: &mut EditorSpace, root: Option<&Path>) {
        self.poll();
        let Some(prepared) = &self.prepared else {
            return;
        };
        let mut apply = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("language-edit-review")).show(ctx, |ui| {
            ui.heading("Review language edits");
            ui.label("Applies to editor buffers. Save separately to write files.");
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for file in &prepared.files {
                        ui.label(file.expected.path.display().to_string());
                        ui.label(egui::RichText::new(&file.summary).monospace());
                    }
                });
            apply = ui.button("Apply reviewed edits").clicked();
            cancel = ui.button("Cancel").clicked();
        });
        if apply {
            if self
                .query
                .as_ref()
                .is_some_and(|q| Some(q.root.as_path()) == root)
            {
                let prepared = self.prepared.take().expect("review");
                self.message = editors
                    .apply_language_edit(prepared)
                    .unwrap_or_else(|| "Edits applied to buffers; save when ready".into());
            } else {
                self.message = "Workspace changed; request edits again".into();
                self.prepared = None;
            }
        }
        if cancel {
            self.prepared = None;
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        editors: &mut EditorSpace,
    ) -> Option<Action> {
        self.poll();
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.visible = false;
            editors.focus_active();
            return None;
        }
        ui.set_min_width(560.0);
        ui.heading("Language Intelligence");
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            for kind in [
                Kind::Completion,
                Kind::Hover,
                Kind::Definition,
                Kind::References,
                Kind::Rename,
                Kind::Signature,
                Kind::DocumentSymbols,
                Kind::WorkspaceSymbols,
                Kind::CodeActions,
            ] {
                if ui.button(kind.label()).clicked() {
                    self.open(kind);
                }
            }
        });
        if let Some(kind) = self.kind
            && matches!(kind, Kind::Rename | Kind::WorkspaceSymbols)
        {
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .char_limit(256)
                        .hint_text(if kind == Kind::Rename {
                            "New name"
                        } else {
                            "Symbol query"
                        }),
                );
                if self.focus {
                    response.request_focus();
                    self.focus = false;
                }
                if ui.button("Request").clicked() {
                    action = Some(Action::Request(kind));
                }
            });
        }
        ui.colored_label(egui::Color32::from(theme.text.secondary), &self.message);
        let mut completion = None;
        let mut edit = None;
        if let Some(result) = &self.result {
            if !result.text.is_empty() {
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        ui.label(&result.text);
                    });
            }
            if let Some(workspace) = &result.edit
                && ui.button("Preview workspace edits").clicked()
            {
                edit = Some(workspace.clone());
            }
            egui::ScrollArea::vertical().max_height(320.0).show_rows(
                ui,
                28.0,
                result.entries.len(),
                |ui, rows| {
                    for index in rows {
                        let entry = &result.entries[index];
                        ui.horizontal(|ui| {
                            let response = ui.add_enabled(
                                entry.disabled.is_none(),
                                egui::Button::new(&entry.label),
                            );
                            if response.clicked() {
                                if let Some(loc) = &entry.location {
                                    action = Some(Action::Navigate(loc.clone()));
                                } else if let Some(item) = &entry.completion {
                                    completion = Some(item.clone());
                                } else if let Some(changes) = &entry.edit {
                                    edit = Some(changes.clone());
                                }
                            }
                            if let Some(reason) = &entry.disabled {
                                response.on_hover_text(reason);
                            }
                            ui.add(egui::Label::new(&entry.detail).truncate());
                        });
                    }
                },
            );
        }
        if let Some(item) = completion
            && let Some(query) = &self.query
            && let Some(snapshot) = &query.source
        {
            let primary = if let Some(primary) = item.edit {
                primary
            } else {
                let mut start = query.caret;
                while start > 0 {
                    let c = snapshot.rope.char(start - 1);
                    if !c.is_alphanumeric() && c != '_' && c != '$' {
                        break;
                    }
                    start -= 1;
                }
                features::TextEdit {
                    range: features::Range {
                        start: buture_lsp::position(snapshot, start).unwrap_or(Value::Null),
                        end: buture_lsp::position(snapshot, query.caret).unwrap_or(Value::Null),
                    },
                    text: item.text,
                }
            };
            let mut all = vec![primary];
            all.extend(item.additional);
            match features::buffer_edits(snapshot, &all) {
                Ok(edits) => {
                    let start = edits[0].range.start;
                    let mut caret = start as isize + edits[0].text.chars().count() as isize;
                    for edit in &edits[1..] {
                        if edit.range.start < start {
                            caret += edit.text.chars().count() as isize - edit.range.len() as isize;
                        }
                    }
                    if let Some(error) = editors.apply_snapshot_edits(snapshot, &edits) {
                        self.message = error;
                    } else {
                        editors.set_caret(caret.max(0) as usize);
                        self.message = "Completion applied".into();
                    }
                }
                Err(e) => self.message = e.to_string(),
            }
        }
        if let Some(edit) = edit {
            self.prepare(edit);
        }
        if ui.button("Close").clicked() {
            self.visible = false;
            editors.focus_active();
        }
        action
    }
}
