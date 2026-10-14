//! Source control reads independently; every mutation originates in a user action.
use crate::Theme;
use buture_git::{GitError, Operation, Plan, Repository, Status};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
type ScanResult = Result<(Repository, Status), GitError>;
#[derive(Default)]
pub struct SourceControl {
    workspace: Option<PathBuf>,
    pub repository: Option<Repository>,
    pub status: Option<Status>,
    scan: Option<mpsc::Receiver<ScanResult>>,
    planning: Option<mpsc::Receiver<Result<Plan, GitError>>>,
    review: Option<Plan>,
    operation: Option<mpsc::Receiver<Result<String, GitError>>>,
    trusted: bool,
    consent: bool,
    pub commit_message: String,
    branch_input: String,
    remote: String,
    pub message: String,
    reported: String,
    refreshed: Option<Instant>,
}
impl SourceControl {
    pub fn set_workspace(&mut self, root: PathBuf) {
        self.scan = None;
        self.planning = None;
        self.workspace = Some(root);
        self.repository = None;
        self.status = None;
        self.trusted = false;
        self.review = None;
        self.refresh();
    }
    pub fn refresh(&mut self) {
        let Some(root) = self.workspace.clone() else {
            return;
        };
        if self.scan.is_some() || self.operation.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.scan = Some(rx);
        std::thread::spawn(move || {
            let result = Repository::discover(&root)
                .and_then(|repo| repo.status().map(|status| (repo, status)));
            let _ = tx.send(result);
        });
    }
    pub fn running(&self) -> bool {
        self.operation.is_some()
    }
    pub fn busy(&self) -> bool {
        self.scan.is_some() || self.planning.is_some() || self.operation.is_some()
    }
    pub fn report(&mut self) -> Option<String> {
        if self.message != self.reported {
            self.reported = self.message.clone();
            Some(self.message.clone())
        } else {
            None
        }
    }
    pub fn poll(&mut self, visible: bool) {
        let scanned = self.scan.as_ref().and_then(|r| r.try_recv().ok());
        if let Some(result) = scanned {
            self.scan = None;
            self.refreshed = Some(Instant::now());
            match result {
                Ok((repo, status)) => {
                    self.repository = Some(repo);
                    if self.remote.is_empty() {
                        self.remote = status
                            .remotes
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "origin".into());
                    }
                    self.status = Some(status);
                }
                Err(e) => self.message = e.to_string(),
            }
        }
        let planned = self.planning.as_ref().and_then(|r| r.try_recv().ok());
        if let Some(result) = planned {
            self.planning = None;
            match result {
                Ok(plan)
                    if self.trusted
                        && self
                            .repository
                            .as_ref()
                            .is_some_and(|repo| repo.root == plan.repository.root) =>
                {
                    if matches!(plan.operation, Operation::Commit(_)) {
                        self.consent = false;
                        self.review = Some(plan);
                    } else {
                        self.operation = Some(plan.confirm().execute());
                    }
                }
                Ok(_) => {
                    self.message =
                        "Trust or repository changed; operation cancelled before execution".into()
                }
                Err(e) => self.message = e.to_string(),
            }
        }
        let finished = self.operation.as_ref().and_then(|r| r.try_recv().ok());
        if let Some(result) = finished {
            self.operation = None;
            self.message = match result {
                Ok(message) => message,
                Err(e) => e.to_string(),
            };
            self.refresh();
        }
        if visible
            && !self.busy()
            && self
                .refreshed
                .is_none_or(|at| at.elapsed() > Duration::from_secs(3))
        {
            self.refresh();
        }
    }
    fn request(&mut self, operation: Operation, dirty: &[PathBuf]) {
        if !self.trusted || self.busy() {
            self.message = "Trust this repository and wait for the current operation".into();
            return;
        }
        let Some(repo) = self.repository.clone() else {
            return;
        };
        if matches!(operation, Operation::Checkout(_) | Operation::Pull(_)) && !dirty.is_empty() {
            self.message = "Save editor changes before updating the working tree".into();
            return;
        }
        if let Operation::Stage(paths) = &operation
            && paths
                .iter()
                .any(|path| dirty.contains(&repo.root.join(path)))
        {
            self.message = "Save this editor buffer before staging its disk contents".into();
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.planning = Some(rx);
        self.message = "Preparing Git operation…".into();
        std::thread::spawn(move || {
            let _ = tx.send(repo.review(operation));
        });
    }
    pub fn review(&mut self, ctx: &egui::Context) {
        let Some(plan) = &self.review else {
            return;
        };
        let mut commit = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("git-commit-review")).show(ctx,|ui|{ui.heading("Review commit");ui.label(format!("Repository: {}",plan.repository.root.display()));if let Operation::Commit(message)=&plan.operation{ui.label(egui::RichText::new(message).monospace());}ui.label("Identity comes from your Git configuration. The reviewed staged tree is committed; no automatic credits or trailers.");if let Some(diff)=plan.staged_preview(){ui.label("Preview limited to 8 KiB. Inspect every changed file before confirming.");let preview=String::from_utf8_lossy(&diff[..diff.len().min(8192)]);egui::ScrollArea::vertical().max_height(300.0).show(ui,|ui|{ui.label(egui::RichText::new(preview.as_ref()).monospace());});}ui.checkbox(&mut self.consent,"I reviewed the staged contents and commit message");commit=ui.add_enabled(self.consent,egui::Button::new("Commit reviewed tree")).clicked();cancel=ui.button("Cancel").clicked();});
        if commit
            && self.trusted
            && let Some(plan) = self.review.take()
        {
            self.operation = Some(plan.confirm().execute());
        }
        if cancel {
            self.review = None;
        }
    }
    pub fn summary(&self, ui: &mut egui::Ui) {
        ui.heading("Source Control");
        if let Some(status) = &self.status {
            ui.label(&status.branch);
            ui.label(format!(
                "{} staged · {} changed",
                status.changes.iter().filter(|c| c.staged()).count(),
                status.changes.iter().filter(|c| c.changed()).count()
            ));
        } else {
            ui.label("Repository status not available");
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme, dirty: &[PathBuf]) -> Option<PathBuf> {
        ui.horizontal(|ui| {
            ui.heading("Source Control");
            if ui
                .add_enabled(!self.busy(), egui::Button::new("Refresh"))
                .clicked()
            {
                self.refresh();
            }
            if self.busy() {
                ui.spinner();
            }
        });
        ui.colored_label(egui::Color32::from(theme.text.secondary), &self.message);
        let Some(repo) = self.repository.clone() else {
            ui.label("Open a Git working tree. No network operation runs automatically.");
            return None;
        };
        ui.label(repo.root.display().to_string());
        ui.checkbox(
            &mut self.trusted,
            "I trust this repository's Git configuration (filters/signing tools may execute)",
        );
        let can = self.trusted && !self.busy();
        let mut selected = None;
        let mut request = None;
        let Some(status) = &self.status else {
            return None;
        };
        ui.horizontal(|ui| {
            ui.label(format!("Branch: {}", status.branch));
            egui::ComboBox::from_id_salt("git-branches")
                .selected_text(if self.branch_input.is_empty() {
                    "Choose branch"
                } else {
                    &self.branch_input
                })
                .show_ui(ui, |ui| {
                    for branch in &status.branches {
                        ui.selectable_value(&mut self.branch_input, branch.clone(), branch);
                    }
                });
            ui.add(
                egui::TextEdit::singleline(&mut self.branch_input)
                    .hint_text("New branch")
                    .char_limit(256)
                    .desired_width(150.0),
            );
            if ui
                .add_enabled(can, egui::Button::new("Create branch"))
                .clicked()
            {
                request = Some(Operation::CreateBranch(self.branch_input.clone()));
            }
            if ui
                .add_enabled(
                    can && !dirty.iter().any(|p| p.starts_with(&repo.root)),
                    egui::Button::new("Checkout"),
                )
                .clicked()
            {
                request = Some(Operation::Checkout(self.branch_input.clone()));
            }
        });
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("git-remote")
                .selected_text(&self.remote)
                .show_ui(ui, |ui| {
                    for remote in &status.remotes {
                        ui.selectable_value(&mut self.remote, remote.clone(), remote);
                    }
                });
            for (label, operation) in [
                ("Fetch", Operation::Fetch(self.remote.clone())),
                ("Pull (ff-only)", Operation::Pull(self.remote.clone())),
                ("Push", Operation::Push(self.remote.clone())),
            ] {
                if ui.add_enabled(can, egui::Button::new(label)).clicked() {
                    request = Some(operation);
                }
            }
        });
        ui.separator();
        ui.label("Commit message");
        ui.add(
            egui::TextEdit::multiline(&mut self.commit_message)
                .desired_rows(3)
                .char_limit(65536)
                .desired_width(ui.available_width()),
        );
        if ui
            .add_enabled(
                can && !self.commit_message.trim().is_empty()
                    && status.changes.iter().any(|c| c.staged()),
                egui::Button::new("Review commit…"),
            )
            .clicked()
        {
            request = Some(Operation::Commit(self.commit_message.clone()));
        }
        if status.partial {
            ui.label("Status reached its file limit; partial list.");
        }
        ui.columns(2, |columns| {
            for (index, staged) in [(0, false), (1, true)] {
                let ui = &mut columns[index];
                let changes: Vec<_> = status
                    .changes
                    .iter()
                    .filter(|change| {
                        if staged {
                            change.staged()
                        } else {
                            change.changed()
                        }
                    })
                    .collect();
                ui.heading(if staged { "Staged Changes" } else { "Changes" });
                egui::ScrollArea::vertical()
                    .id_salt(("git-changes", staged))
                    .max_height(400.0)
                    .show_rows(ui, 28.0, changes.len(), |ui, rows| {
                        for row in rows {
                            let change = changes[row];
                            if (staged && !change.staged()) || (!staged && !change.changed()) {
                                continue;
                            }
                            ui.horizontal(|ui| {
                                let label = format!(
                                    "{}{} {}",
                                    change.index,
                                    change.worktree,
                                    change.path.display()
                                );
                                if ui.add(egui::Button::new(label).frame(false)).clicked() {
                                    selected = Some(repo.root.join(&change.path));
                                }
                                if ui
                                    .add_enabled(
                                        can,
                                        egui::Button::new(if staged { "Unstage" } else { "Stage" }),
                                    )
                                    .clicked()
                                {
                                    let mut paths = vec![change.path.clone()];
                                    if let Some(old) = &change.old_path {
                                        paths.push(old.clone());
                                    }
                                    request = Some(if staged {
                                        Operation::Unstage(paths)
                                    } else {
                                        Operation::Stage(paths)
                                    });
                                }
                            });
                        }
                    });
            }
        });
        if let Some(operation) = request {
            self.request(operation, dirty);
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revoking_trust_cancels_a_prepared_mutation_without_staging() {
        let Ok(repo) = Repository::discover(&std::env::current_dir().expect("cwd")) else {
            return;
        };
        let plan = repo
            .review(Operation::Stage(vec!["README.md".into()]))
            .expect("review");
        let (tx, rx) = mpsc::channel();
        let mut panel = SourceControl {
            repository: Some(repo),
            planning: Some(rx),
            ..Default::default()
        };
        tx.send(Ok(plan)).expect("result");
        panel.poll(false);
        assert!(panel.operation.is_none());
        assert!(panel.message.contains("cancelled"));
    }
    #[test]
    fn separate_changed_and_staged_lists_use_their_filtered_lengths() {
        let Ok(repo) = Repository::discover(&std::env::current_dir().expect("cwd")) else {
            return;
        };
        let mut panel = SourceControl {
            repository: Some(repo),
            status: Some(Status {
                head: None,
                branch: "fixture".into(),
                changes: vec![buture_git::Change {
                    path: "fixture.txt".into(),
                    old_path: None,
                    index: '?',
                    worktree: '?',
                }],
                branches: vec![],
                remotes: vec![],
                partial: false,
            }),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, &Theme::dark(), &[]));
        });
    }
}
