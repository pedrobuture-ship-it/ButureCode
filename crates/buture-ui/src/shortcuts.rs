//! User profile editor validates before the IO worker commits any change.
use crate::Theme;
use buture_core::{
    commands::CommandRegistry,
    keybindings::{Keymap, Platform},
};
use egui::{RichText, Ui};
#[derive(Default)]
pub struct Shortcuts {
    pub text: String,
    pub error: Option<String>,
}
pub enum Action {
    Save,
    Reload,
}
impl Shortcuts {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        registry: &CommandRegistry,
        keymap: &Keymap,
    ) -> Option<Action> {
        ui.heading("Keyboard Shortcuts");
        ui.label("User overrides. Workspace files cannot change shortcuts.");
        ui.label("command.id = Primary+Key · none disables a command · comma separates aliases");
        ui.label("Primary means Cmd on macOS and Ctrl on Windows/Linux. A space separates a two-stroke chord.");
        let response = ui.add(
            egui::TextEdit::multiline(&mut self.text)
                .font(egui::TextStyle::Monospace)
                .desired_rows(10)
                .char_limit(64 * 1024)
                .desired_width(ui.available_width()),
        );
        let validation = Keymap::overrides(&self.text, Platform::current(), registry);
        let mut action = None;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(validation.is_ok(), egui::Button::new("Save profile"))
                .clicked()
            {
                action = Some(Action::Save);
            }
            if ui.button("Reload profile").clicked() {
                action = Some(Action::Reload);
            }
            if ui.button("Restore defaults in editor").clicked() {
                self.text =
                    "# Buture Code user shortcuts\n# Defaults apply unless overridden below.\n"
                        .into();
                self.error = None;
            }
        });
        if response.changed() {
            self.error = None;
        }
        if let Err(error) = validation {
            ui.label(RichText::new(error.to_string()).color(theme.semantic.warning));
        }
        if let Some(error) = &self.error {
            ui.label(RichText::new(error).color(theme.semantic.warning));
        }
        ui.separator();
        ui.label("Active shortcuts");
        egui::ScrollArea::vertical()
            .id_salt("active-shortcuts")
            .show(ui, |ui| {
                for command in &registry.commands {
                    let label = keymap.label(command.id);
                    if !label.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(command.id).monospace());
                            ui.label(format!("{} · {label}", command.label));
                        });
                    }
                }
            });
        action
    }
}
