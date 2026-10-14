//! Formatter consent is tied to executable/provider/root. Automatic saves retain a safe fallback.
use crate::{Theme, editor::EditorSpace};
use buture_editor::Buffer;
use buture_format::{FormatError, Formatted, Kind, Plan};
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::mpsc,
};
enum Mode {
    Manual,
    Save(Buffer),
}
struct Planning {
    mode: Mode,
    receiver: mpsc::Receiver<Result<Plan, FormatError>>,
}
struct Running {
    mode: Mode,
    receiver: mpsc::Receiver<Result<Formatted, FormatError>>,
}
#[derive(Default)]
pub struct Formatting {
    pub visible: bool,
    provider: Option<Kind>,
    executable: String,
    trusted: BTreeMap<(PathBuf, Kind), PathBuf>,
    planning: Option<Planning>,
    running: Option<Running>,
    review: Option<Plan>,
    consent: bool,
    queue: VecDeque<(Buffer, PathBuf)>,
    pub message: String,
    reported: String,
}
impl Formatting {
    pub fn take_report(&mut self) -> Option<String> {
        if self.message != self.reported {
            self.reported = self.message.clone();
            Some(self.message.clone())
        } else {
            None
        }
    }
    pub fn reset_trust(&mut self) {
        self.trusted.clear();
        self.review = None;
        self.consent = false;
    }
    fn prepare(
        &mut self,
        buffer: Buffer,
        selection: Option<std::ops::Range<usize>>,
        root: PathBuf,
        mode: Mode,
    ) {
        let provider = self.provider;
        let executable = (!self.executable.is_empty()).then(|| PathBuf::from(&self.executable));
        let (tx, rx) = mpsc::channel();
        self.planning = Some(Planning { mode, receiver: rx });
        self.message = "Preparing formatter command…".into();
        std::thread::spawn(move || {
            let _ = tx.send(Plan::review(buffer, selection, provider, executable, &root));
        });
    }
    pub fn request(&mut self, editors: &EditorSpace, root: Option<&Path>, selection: bool) {
        self.visible = true;
        if self.planning.is_some() || self.running.is_some() || self.review.is_some() {
            self.message = "Wait for the current formatter operation".into();
            return;
        }
        let Some(root) = root else {
            self.message = "Open a workspace first".into();
            return;
        };
        let Some(buffer) = editors.active_buffer() else {
            self.message = "Open a document first".into();
            return;
        };
        self.prepare(
            buffer,
            selection.then(|| editors.selection_range()),
            root.into(),
            Mode::Manual,
        );
    }
    pub fn collect_saves(&mut self, editors: &mut EditorSpace, root: Option<&Path>) {
        for buffer in editors.take_format_saves() {
            let kind = self
                .provider
                .or_else(|| buffer.path.as_ref().and_then(|p| Kind::infer(p)));
            if let (Some(root), Some(kind)) = (root, kind)
                && self.trusted.contains_key(&(root.into(), kind))
                && self.queue.len() < 64
            {
                self.queue.push_back((buffer, root.into()));
            } else {
                self.message="Format on Save skipped: no trusted formatter for this root/provider (or queue limit); original save requested".into();
                editors.resume_original_save(buffer);
            }
        }
    }
    pub fn poll(&mut self, editors: &mut EditorSpace) {
        let planned = self
            .planning
            .as_ref()
            .and_then(|p| p.receiver.try_recv().ok());
        if let Some(result) = planned {
            let planning = self.planning.take().expect("planning");
            match (planning.mode, result) {
                (Mode::Manual, Ok(plan)) => {
                    self.review = Some(plan);
                    self.consent = false;
                    self.message = "Review formatter execution".into();
                }
                (Mode::Manual, Err(error)) => self.message = error.to_string(),
                (Mode::Save(buffer), Ok(plan)) => {
                    if self.trusted.get(&(plan.root.clone(), plan.kind)) == Some(&plan.executable) {
                        self.running = Some(Running {
                            mode: Mode::Save(buffer),
                            receiver: plan.confirm().execute(),
                        });
                    } else {
                        editors.resume_original_save(buffer);
                        self.message =
                            "Formatter configuration changed; original save requested".into();
                    }
                }
                (Mode::Save(buffer), Err(error)) => {
                    editors.resume_original_save(buffer);
                    self.message = format!("{error}; original save requested");
                }
            }
        }
        let finished = self
            .running
            .as_ref()
            .and_then(|r| r.receiver.try_recv().ok());
        if let Some(result) = finished {
            match self.running.take().expect("running").mode {
                Mode::Manual => match result {
                    Ok(result) => {
                        self.message = editors
                            .apply_snapshot_edits(
                                &result.snapshot,
                                std::slice::from_ref(&result.edit),
                            )
                            .unwrap_or_else(|| {
                                "Formatter completed for the requested buffer".into()
                            })
                    }
                    Err(e) => self.message = e.to_string(),
                },
                Mode::Save(original) => {
                    if let Some(message) = editors.finish_format_save(original, result) {
                        self.message = message;
                    } else {
                        self.message = "Formatted snapshot queued for saving".into();
                    }
                }
            }
        }
        if self.running.is_none()
            && self.planning.is_none()
            && self.review.is_none()
            && let Some((buffer, root)) = self.queue.pop_front()
        {
            self.prepare(buffer.clone(), None, root, Mode::Save(buffer));
        }
    }
    pub fn review(&mut self, ctx: &egui::Context) {
        let Some(plan) = self.review.as_ref() else {
            return;
        };
        let mut execute = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("formatter-review")).show(ctx,|ui|{ui.heading("Review formatter");ui.label(format!("{} {:?}",plan.executable.display(),plan.args));ui.label(format!("cwd: {}",plan.root.display()));ui.label(format!("Source: {} · character range {:?}",plan.snapshot.path.display(),plan.range));ui.label("Local tools may load project configuration/plugins. No write-in-place flags; result is applied to your buffer.");ui.checkbox(&mut self.consent,"I trust this workspace and this formatter executable");execute=ui.add_enabled(self.consent,egui::Button::new("Execute reviewed formatter")).clicked();cancel=ui.button("Cancel").clicked();});
        if execute {
            let plan = self.review.take().expect("reviewed plan");
            self.trusted
                .insert((plan.root.clone(), plan.kind), plan.executable.clone());
            self.running = Some(Running {
                mode: Mode::Manual,
                receiver: plan.confirm().execute(),
            });
            self.review = None;
        }
        if cancel {
            self.review = None;
            self.message = "Formatter review cancelled".into();
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        editors: &EditorSpace,
        root: Option<&Path>,
        on_save: &mut bool,
    ) -> Option<bool> {
        ui.heading("Formatting");
        ui.label(
            "Start with a reviewed Format Document command to trust a provider in this workspace.",
        );
        ui.label("Without trust, Format on Save saves your original text. Tool failures and stale results also preserve the original save request.");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("formatter-provider")
                .selected_text(
                    self.provider
                        .map(|p| p.executable())
                        .unwrap_or("Auto by language"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.provider, None, "Auto by language");
                    for kind in Kind::ALL {
                        ui.selectable_value(&mut self.provider, Some(kind), kind.executable());
                    }
                });
            ui.label("Executable override (absolute, optional)");
            if ui
                .add(egui::TextEdit::singleline(&mut self.executable).char_limit(4096))
                .changed()
            {
                self.reset_trust();
            }
        });
        let changed = ui
            .checkbox(on_save, "Format on Save (user preference)")
            .changed();
        let mut request = None;
        ui.horizontal(|ui| {
            if ui.button("Format Document").clicked() {
                request = Some(false);
            }
            if ui.button("Format Selection").clicked() {
                request = Some(true);
            }
        });
        if let Some(selection) = request {
            self.request(editors, root, selection);
        }
        if self.planning.is_some() || self.running.is_some() {
            ui.spinner();
        }
        ui.colored_label(egui::Color32::from(theme.text.secondary), &self.message);
        changed.then_some(*on_save)
    }
}
