//! Explicit local language-server startup; snapshots are serialized in the LSP worker.
use crate::{Theme, editor::EditorSpace};
use buture_lsp::{Client, Event, Server};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
pub type DiagnosticRecord = (
    std::sync::Arc<Vec<buture_lsp::features::Diagnostic>>,
    Option<(u64, u64)>,
);
#[derive(Default)]
pub struct Language {
    pub client: Option<Client>,
    pub intelligence: crate::intelligence::Intelligence,
    pub diagnostics: std::collections::BTreeMap<PathBuf, DiagnosticRecord>,
    pub capabilities: serde_json::Value,
    pub state: String,
    problems: crate::problems::Problems,
    diagnostic_revision: u64,
    executable: String,
    consent: bool,
    root: Option<PathBuf>,
    detected: Option<mpsc::Receiver<Option<PathBuf>>>,
    signatures: Vec<(PathBuf, u64, u64)>,
}
impl Language {
    pub fn reset_workspace(&mut self) {
        if let Some(client) = &self.client {
            client.shutdown();
        }
        self.consent = false;
        self.diagnostics.clear();
        self.diagnostic_revision += 1;
        self.signatures.clear();
    }
    pub fn poll(&mut self, editors: &mut EditorSpace) {
        if let Some(rx) = &self.detected
            && let Ok(path) = rx.try_recv()
        {
            self.executable = path.map(|p| p.display().to_string()).unwrap_or_default();
            self.state = if self.executable.is_empty() {
                "rust-analyzer executable not found".into()
            } else {
                "Executable detected; not initialized".into()
            };
            self.detected = None;
        }
        let saved = editors.take_saved_snapshots();
        let mut stopped = false;
        if let Some(client) = &self.client {
            for event in client.events.try_iter().take(256) {
                match event {
                    Event::Ready(capabilities) => {
                        self.capabilities = capabilities;
                        self.state = "Language server initialized · UTF-16 positions".into();
                        self.signatures.clear();
                    }
                    Event::Warning(e) => {
                        self.state = e.to_string();
                        self.intelligence.error(e.to_string());
                    }
                    Event::Stopped => {
                        self.state = format!("{} · stopped", self.state);
                        stopped = true;
                    }
                    Event::Feature {
                        kind,
                        result,
                        context,
                    } => self.intelligence.receive(kind, result, context),
                    Event::Diagnostics {
                        path,
                        items,
                        signature,
                        markers,
                    } => {
                        editors.set_markers(&path, signature, markers);
                        let used: usize = self
                            .diagnostics
                            .iter()
                            .filter(|(p, _)| *p != &path)
                            .map(|(_, record)| record.0.len())
                            .sum();
                        let mut items = items;
                        let allowed = 10000usize.saturating_sub(used);
                        if items.len() > allowed {
                            items.truncate(allowed);
                            self.state =
                                "Diagnostics cache reached 10,000 items; partial results".into();
                        }
                        if self.diagnostics.len() < 512 || self.diagnostics.contains_key(&path) {
                            self.diagnostics
                                .insert(path, (std::sync::Arc::new(items), signature));
                            self.diagnostic_revision += 1;
                        }
                    }
                    Event::Response { .. } | Event::Notification { .. } => {}
                }
            }
            if !stopped {
                let all = editors.open_snapshots();
                let signatures = all
                    .iter()
                    .map(|s| (s.path.clone(), s.revision, s.content_id))
                    .collect::<Vec<_>>();
                if signatures != self.signatures || !saved.is_empty() {
                    match client.sync(all, saved) {
                        Ok(()) => self.signatures = signatures,
                        Err(e) => self.state = e.to_string(),
                    }
                }
            }
        }
        if stopped {
            self.client = None;
            self.capabilities = serde_json::Value::Null;
            self.signatures.clear();
        }
    }
    pub fn show_problems(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
    ) -> Option<buture_lsp::features::Location> {
        let data = self
            .diagnostics
            .values()
            .map(|record| record.0.clone())
            .collect();
        self.problems
            .show(ui, theme, self.diagnostic_revision, data)
    }
    pub fn request_feature(&mut self, kind: buture_lsp::features::Kind, editors: &EditorSpace) {
        if let (Some(client), Some(root)) = (&self.client, &self.root) {
            self.intelligence
                .request(kind, client, &self.capabilities, editors, root);
        } else {
            self.intelligence
                .error("Start a local language server first".into());
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme, root: Option<&Path>) {
        ui.heading("Language Servers");
        ui.label("Start a local server explicitly. The workspace is code, and this is not a process sandbox.");
        ui.label("Rust-analyzer: check on save, build scripts and proc-macros disabled; Cargo offline, no dependency fetching.");
        ui.label(format!(
            "Workspace: {}",
            root.map(|p| p.display().to_string())
                .unwrap_or_else(|| "Open a folder first".into())
        ));
        ui.horizontal(|ui| {
            ui.label("Executable");
            ui.add(
                egui::TextEdit::singleline(&mut self.executable)
                    .desired_width(440.0)
                    .char_limit(4096),
            );
            if ui
                .add_enabled(
                    self.client.is_none() && self.detected.is_none(),
                    egui::Button::new("Detect rust-analyzer"),
                )
                .clicked()
            {
                let (tx, rx) = mpsc::channel();
                self.detected = Some(rx);
                std::thread::spawn(move || {
                    let host = buture_toolchains::Host::detect();
                    let path = buture_toolchains::engine::resolve("rust-analyzer", &host.path)
                        .into_iter()
                        .next();
                    let _ = tx.send(path);
                });
            }
        });
        ui.checkbox(
            &mut self.consent,
            "I trust this workspace and the selected local executable",
        );
        if ui
            .add_enabled(
                self.client.is_none()
                    && self.consent
                    && root.is_some()
                    && !self.executable.is_empty(),
                egui::Button::new("Start language server"),
            )
            .clicked()
            && let Some(root) = root
        {
            let server = Server {
                process: buture_protocol::Spec {
                    executable: self.executable.clone().into(),
                    args: vec![],
                    cwd: root.into(),
                },
                language: "rust".into(),
                initialization_options: serde_json::json!({"cargo":{"buildScripts":{"enable":false},"noDeps":true,"metadataExtraArgs":["--offline"]},"checkOnSave":false,"procMacro":{"enable":false},"cachePriming":{"enable":false}}),
            };
            self.root = Some(root.into());
            self.signatures.clear();
            self.state = "Initializing language server…".into();
            self.client = Some(server.confirm().start());
        }
        if let Some(client) = &self.client
            && ui.button("Shutdown server").clicked()
        {
            client.shutdown();
            self.state = "Stopping language server…".into();
        }
        ui.colored_label(egui::Color32::from(theme.text.secondary), &self.state);
        ui.label("Only synchronized UTF-8 documents up to 2 MiB, at most 64 open documents. No automatic install.");
    }
}
