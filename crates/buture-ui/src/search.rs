//! Search UI streams observations and reviews workspace replacements before applying them.
use crate::{Theme, editor::EditorSpace};
use buture_editor::{Edit, TextSnapshot};
use buture_search::{Applied, Event, Hit, Job, Plan, Query, Scope, Stats};
use egui::{Context, RichText, Ui};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
pub struct OpenHit {
    pub hit: Hit,
    pub stamp: Option<(u64, u64)>,
}
type DocumentEdits = Result<(TextSnapshot, Vec<Edit>), buture_search::SearchError>;
pub struct SearchPanel {
    pub text: String,
    pub replacement: String,
    pub workspace: bool,
    pub focus: bool,
    regex: bool,
    case_sensitive: bool,
    whole_word: bool,
    include: String,
    exclude: String,
    job: Option<Job>,
    pub hits: Vec<Hit>,
    stats: Option<Stats>,
    error: Option<String>,
    last_signature: String,
    changed: Instant,
    stamp: Option<(u64, u64)>,
    plan_receiver: Option<mpsc::Receiver<Result<Plan, buture_search::SearchError>>>,
    plan: Option<Plan>,
    confirmed: bool,
    plan_label: String,
    apply_receiver: Option<mpsc::Receiver<Applied>>,
    document_receiver: Option<mpsc::Receiver<DocumentEdits>>,
}
impl Default for SearchPanel {
    fn default() -> Self {
        Self {
            text: String::new(),
            replacement: String::new(),
            workspace: false,
            focus: false,
            regex: false,
            case_sensitive: false,
            whole_word: false,
            include: String::new(),
            exclude: String::new(),
            job: None,
            hits: Vec::new(),
            stats: None,
            error: None,
            last_signature: String::new(),
            changed: Instant::now(),
            stamp: None,
            plan_receiver: None,
            plan: None,
            confirmed: false,
            plan_label: String::new(),
            apply_receiver: None,
            document_receiver: None,
        }
    }
}
impl SearchPanel {
    pub fn busy(&self) -> bool {
        self.job.is_some()
            || self.plan_receiver.is_some()
            || self.apply_receiver.is_some()
            || self.document_receiver.is_some()
    }
    fn query(&self) -> Query {
        Query {
            text: self.text.clone(),
            regex: self.regex,
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            include: self
                .include
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            exclude: self
                .exclude
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
        }
    }
    pub fn open(&mut self, workspace: bool) {
        self.workspace = workspace;
        self.focus = true;
        self.changed = Instant::now();
    }
    pub fn poll(&mut self, editors: &mut EditorSpace) -> Option<String> {
        let mut done = false;
        if let Some(job) = &self.job {
            for _ in 0..256 {
                match job.events.try_recv() {
                    Ok(Event::Hit(hit)) => self.hits.push(hit),
                    Ok(Event::Done(stats)) => {
                        self.stats = Some(stats);
                        done = true;
                        break;
                    }
                    Ok(Event::Failed(e)) => {
                        self.error = Some(e.to_string());
                        done = true;
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
        if done {
            self.job = None;
        }
        if let Some(rx) = &self.plan_receiver
            && let Ok(result) = rx.try_recv()
        {
            match result {
                Ok(plan) => self.plan = Some(plan),
                Err(e) => self.error = Some(e.to_string()),
            };
            self.plan_receiver = None;
        }
        if let Some(rx) = &self.apply_receiver
            && let Ok(applied) = rx.try_recv()
        {
            self.apply_receiver = None;
            self.last_signature.clear();
            let message = if let Some(error) = applied.failure {
                format!(
                    "{} file(s) applied before failure: {error}",
                    applied.files.len()
                )
            } else {
                format!(
                    "Replacements applied to {} file(s). Rescan to inspect the result.",
                    applied.files.len()
                )
            };
            editors.reload_clean(&applied.files);
            return Some(message);
        }
        if let Some(rx) = &self.document_receiver
            && let Ok(result) = rx.try_recv()
        {
            self.document_receiver = None;
            match result {
                Ok((snapshot, edits)) => {
                    if let Some(error) = editors.apply_snapshot_edits(&snapshot, &edits) {
                        self.error = Some(error);
                    } else {
                        self.last_signature.clear();
                    }
                }
                Err(e) => self.error = Some(e.to_string()),
            };
        }
        None
    }
    fn start(&mut self, root: Option<&PathBuf>, snapshot: Option<TextSnapshot>) {
        let scope = if self.workspace {
            let Some(root) = root else {
                self.error = Some("Open a workspace first".into());
                return;
            };
            self.stamp = None;
            Scope::Workspace(root.clone())
        } else {
            let Some(snapshot) = snapshot else {
                self.error = Some("Open a text file first".into());
                return;
            };
            self.stamp = Some((snapshot.revision, snapshot.content_id));
            Scope::Document(snapshot)
        };
        self.hits.clear();
        self.stats = None;
        self.error = None;
        self.job = Some(Job::start(scope, self.query()));
    }
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        root: Option<&PathBuf>,
        editors: &EditorSpace,
    ) -> Option<OpenHit> {
        ui.heading("Search");
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.text)
                .hint_text("Find text or regex…")
                .desired_width(ui.available_width()),
        );
        if self.focus {
            response.request_focus();
            self.focus = false;
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.workspace, "Workspace");
            ui.checkbox(&mut self.case_sensitive, "Match case");
            ui.checkbox(&mut self.whole_word, "Whole word");
            ui.checkbox(&mut self.regex, "Regex");
        });
        if self.workspace {
            ui.horizontal(|ui| {
                ui.label("Include globs");
                ui.text_edit_singleline(&mut self.include);
                ui.label("Exclude");
                ui.text_edit_singleline(&mut self.exclude);
            });
        }
        let snapshot = editors.active_snapshot();
        let signature = format!(
            "{:?} {} {:?} {:?}",
            self.query(),
            self.workspace,
            root,
            if self.workspace {
                None
            } else {
                snapshot
                    .as_ref()
                    .map(|s| (&s.path, s.revision, s.content_id))
            }
        );
        if signature != self.last_signature {
            self.last_signature = signature;
            self.changed = Instant::now();
            self.job = None;
            self.hits.clear();
            self.stats = None;
            self.error = None;
        }
        let mut search = false;
        ui.horizontal(|ui| {
            if ui.button("Search").clicked() {
                search = true;
            }
            ui.label("Replace");
            ui.text_edit_singleline(&mut self.replacement);
            if ui
                .add_enabled(
                    !self.hits.is_empty()
                        && self.plan_receiver.is_none()
                        && self.document_receiver.is_none(),
                    egui::Button::new(if self.workspace {
                        "Review workspace replacement…"
                    } else {
                        "Replace all in document"
                    }),
                )
                .clicked()
            {
                let query = self.query();
                let replacement = self.replacement.clone();
                if self.workspace
                    && let Some(root) = root
                {
                    let root = root.clone();
                    let paths = self.hits.iter().map(|h| h.path.clone()).collect::<Vec<_>>();
                    let dirty = editors.dirty_paths();
                    let (tx, rx) = mpsc::channel();
                    self.plan_receiver = Some(rx);
                    self.confirmed = false;
                    self.plan_label = format!("{} → {}", self.text, self.replacement);
                    std::thread::spawn(move || {
                        let _ = tx.send(Plan::prepare(&root, &paths, &query, &replacement, &dirty));
                    });
                } else if let Some(snapshot) = snapshot.clone() {
                    let (tx, rx) = mpsc::channel();
                    self.document_receiver = Some(rx);
                    std::thread::spawn(move || {
                        let result = (|| {
                            let buffer =
                                buture_editor::Buffer::from_text(&snapshot.rope.to_string());
                            let edits =
                                buture_search::replacement_edits(&buffer, &query, &replacement)?;
                            Ok((snapshot, edits))
                        })();
                        let _ = tx.send(result);
                    });
                }
            }
            if self.job.is_some()
                || self.plan_receiver.is_some()
                || self.apply_receiver.is_some()
                || self.document_receiver.is_some()
            {
                ui.spinner();
            }
        });
        if search
            || (!self.text.is_empty()
                && self.job.is_none()
                && self.stats.is_none()
                && self.error.is_none()
                && self.changed.elapsed() > Duration::from_millis(180))
        {
            self.start(root, snapshot);
        }
        if let Some(stats) = &self.stats {
            ui.label(format!(
                "{} matches · {} files searched · {} skipped{}",
                stats.matches,
                stats.files,
                stats.skipped,
                if stats.truncated {
                    " · partial result"
                } else {
                    ""
                }
            ));
        }
        if let Some(error) = &self.error {
            ui.label(RichText::new(error).color(theme.semantic.warning));
        }
        ui.separator();
        let mut selected = None;
        egui::ScrollArea::vertical()
            .id_salt("search-results")
            .show_rows(ui, 28.0, self.hits.len(), |ui, range| {
                for index in range {
                    let hit = &self.hits[index];
                    let relative = root
                        .and_then(|r| hit.path.strip_prefix(r).ok())
                        .unwrap_or(&hit.path);
                    if ui
                        .selectable_label(
                            false,
                            format!(
                                "{}:{}:{}  {}",
                                relative.display(),
                                hit.line,
                                hit.column,
                                hit.preview
                            ),
                        )
                        .clicked()
                    {
                        selected = Some(OpenHit {
                            hit: hit.clone(),
                            stamp: self.stamp,
                        });
                    }
                }
            });
        selected
    }
    pub fn review(&mut self, ctx: &Context, editors: &EditorSpace) {
        if self.plan.is_none() {
            return;
        }
        let mut execute = false;
        let mut cancel = false;
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        egui::Window::new("Review workspace replacements")
            .collapsible(false)
            .default_width(640.0)
            .show(ctx, |ui| {
                ui.heading(&self.plan_label);
                ui.label(format!(
                    "{} file(s). Open documents with unsaved changes are protected.",
                    plan.changes.len()
                ));
                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        for change in &plan.changes {
                            ui.label(format!(
                                "{} · {} replacement(s)",
                                change.path.display(),
                                change.matches
                            ));
                            ui.label(
                                RichText::new(format!("Before: {}", change.before)).monospace(),
                            );
                            ui.label(RichText::new(format!("After: {}", change.after)).monospace());
                            ui.separator();
                        }
                    });
                let dirty = editors.dirty_paths();
                let blocked = plan.changes.iter().any(|c| dirty.contains(&c.path));
                if blocked {
                    ui.label("Save or close modified documents before applying this plan.");
                }
                ui.checkbox(
                    &mut self.confirmed,
                    "I authorize these replacements in the listed files",
                );
                execute = ui
                    .add_enabled(
                        self.confirmed && !blocked && self.apply_receiver.is_none(),
                        egui::Button::new("Apply approved replacements"),
                    )
                    .clicked();
                cancel = ui.button("Cancel").clicked();
            });
        if execute && let Some(plan) = self.plan.take() {
            let (tx, rx) = mpsc::channel();
            self.apply_receiver = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(plan.confirm().apply());
            });
        }
        if cancel {
            self.plan = None;
            self.confirmed = false;
        }
    }
}
