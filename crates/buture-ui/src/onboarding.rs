//! First-run flow; scan data comes from background environment discovery.
use buture_toolchains::Host;
use egui::{RichText, Ui};
#[derive(Default)]
pub struct Onboarding {
    pub step: usize,
    pub languages: Vec<String>,
    pub scanned: bool,
}
pub enum Action {
    Scan,
    Complete,
}
impl Onboarding {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        host: Option<&Host>,
        summary: Option<&str>,
    ) -> Option<Action> {
        ui.label(RichText::new(format!("{} / 5", self.step + 1)).small());
        ui.add_space(16.0);
        match self.step {
            0 => {
                ui.heading("Welcome to Buture Code");
                ui.label("Engineering without friction.");
            }
            1 => {
                ui.heading("System detected");
                if let Some(h) = host {
                    ui.label(format!("{:?} · {:?}", h.os, h.architecture));
                    ui.label(
                        h.shell
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "Shell not provided by host".into()),
                    );
                } else {
                    ui.spinner();
                }
            }
            2 => {
                ui.heading("What do you develop with?");
                egui::Grid::new("onboarding-languages")
                    .num_columns(3)
                    .spacing([12.0, 12.0])
                    .show(ui, |ui| {
                        for (index, language) in [
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
                        .into_iter()
                        .enumerate()
                        {
                            let selected = self.languages.iter().any(|l| l == language);
                            if ui
                                .add_sized(
                                    [125.0, 42.0],
                                    egui::Button::new(language).selected(selected),
                                )
                                .clicked()
                            {
                                if selected {
                                    self.languages.retain(|l| l != language);
                                } else {
                                    self.languages.push(language.into());
                                }
                            }
                            if index % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
            }
            3 => {
                ui.heading("Scan Environment");
                ui.label("Read-only discovery. No tools will be installed.");
                if ui.button("Scan").clicked() {
                    return Some(Action::Scan);
                }
                if summary.is_some() {
                    self.scanned = true;
                }
            }
            _ => {
                ui.heading("Environment Summary");
                if let Some(summary) = summary {
                    ui.label(summary);
                } else {
                    ui.label("No scan results available.");
                }
                if ui.button("Finish").clicked() {
                    return Some(Action::Complete);
                }
            }
        }
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.step > 0, egui::Button::new("Back"))
                .clicked()
            {
                self.step -= 1;
            }
            if ui
                .add_enabled(
                    self.step < 4
                        && (self.step != 1 || host.is_some())
                        && (self.step != 3 || self.scanned),
                    egui::Button::new("Continue"),
                )
                .clicked()
            {
                self.step += 1;
            }
        });
        None
    }
}
