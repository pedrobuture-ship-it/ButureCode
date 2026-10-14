//! Optional project overview: manifest data and observed environment; execution stays unavailable.
use buture_project::Detection;
use egui::Ui;
use std::path::{Path, PathBuf};
pub enum Action {
    Doctor,
    Toolchains,
    Run,
    Tasks,
    Terminal,
    Open(PathBuf),
}
pub fn show(
    ui: &mut Ui,
    root: Option<&Path>,
    detection: Option<&Detection>,
    branch: Option<&str>,
    scanning: bool,
) -> Option<Action> {
    ui.heading("Project Overview");
    let mut action = None;
    if let Some(root) = root {
        ui.label(root.display().to_string());
    } else {
        ui.label("Open a folder to inspect its project structure.");
    }
    ui.horizontal(|ui| {
        if ui.button("Doctor").clicked() {
            action = Some(Action::Doctor);
        }
        if ui.button("Toolchains").clicked() {
            action = Some(Action::Toolchains);
        }
        if ui.button("RUN").clicked() {
            action = Some(Action::Run);
        }
        if ui.button("Build / Test tasks").clicked() {
            action = Some(Action::Tasks);
        }
        if ui.button("Terminal").clicked() {
            action = Some(Action::Terminal);
        }
        ui.add_enabled(false, egui::Button::new("Architecture"))
            .on_hover_text("This action is not available yet.");
    });
    ui.separator();
    ui.label(format!(
        "Branch: {}",
        branch.unwrap_or("No branch detected")
    ));
    ui.label("Build: not run · Tests: not run");
    ui.label("Environment and services: inspect Doctor for observed checks.");
    if scanning {
        ui.spinner();
    }
    if let Some(detection) = detection {
        if detection.projects.is_empty() {
            ui.label("No recognized project manifest was found within scan limits.");
        }
        if detection.truncated {
            ui.label("Project scan reached its limit; this is a partial result.");
        }
        egui::ScrollArea::vertical()
            .id_salt("project-overview")
            .show(ui, |ui| {
                for project in &detection.projects {
                    ui.add_space(16.0);
                    ui.heading(&project.name);
                    ui.label(format!(
                        "{:?} · {} · {:?}",
                        project.kind,
                        project.languages.join(", "),
                        project.build_system
                    ));
                    ui.label(
                        project
                            .manifest
                            .strip_prefix(root.unwrap_or(Path::new("")))
                            .unwrap_or(&project.manifest)
                            .display()
                            .to_string(),
                    );
                    if ui.small_button("Open manifest").clicked() {
                        action = Some(Action::Open(project.manifest.clone()));
                    }
                    if !project.workspace_members.is_empty() {
                        ui.label(format!(
                            "Workspace members: {}",
                            project.workspace_members.join(", ")
                        ));
                    }
                    for candidate in &project.run_candidates {
                        ui.label(format!(
                            "Run candidate: {} · requires workspace trust",
                            candidate.label
                        ));
                    }
                    ui.separator();
                }
                for warning in &detection.warnings {
                    ui.label(warning);
                }
            });
    }
    action
}
