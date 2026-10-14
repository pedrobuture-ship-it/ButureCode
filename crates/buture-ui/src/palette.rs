//! Floating command picker shared with Quick Open presentation.
use crate::Theme;
use buture_core::commands::{Action, CommandRegistry, Scope};
use egui::{Key, RichText, Ui};
#[derive(Default)]
pub struct Palette {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    pub focus: bool,
}
#[derive(Clone, Copy)]
pub struct Availability {
    pub workspace: bool,
    pub editor: bool,
    pub quick_open: bool,
    pub language: bool,
}
impl Availability {
    pub fn reason(self, action: Action, scope: Scope) -> Option<&'static str> {
        match action {
            Action::Unavailable(reason) => Some(reason),
            Action::LanguageFeature(_) if !self.language => Some("Start a language server first"),
            Action::QuickOpen if !self.quick_open => Some("File navigation is not available yet"),
            Action::SearchWorkspace | Action::QuickOpen if !self.workspace => {
                Some("Open a folder first")
            }
            _ if scope == Scope::Editor && !self.editor => Some("Open a text document first"),
            _ => None,
        }
    }
}
impl Palette {
    pub fn activate(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.focus = true;
    }
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        registry: &CommandRegistry,
        availability: Availability,
    ) -> Option<Action> {
        ui.set_min_width(560.0);
        ui.label(RichText::new("Command Palette").strong());
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text("Search commands…")
                .desired_width(ui.available_width()),
        );
        if self.focus {
            response.request_focus();
            self.focus = false;
        }
        if response.changed() {
            self.selected = 0;
        }
        let results = registry.search(&self.query);
        self.selected = self.selected.min(results.len().saturating_sub(1));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown)) {
            self.selected = (self.selected + 1).min(results.len().saturating_sub(1));
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp)) {
            self.selected = self.selected.saturating_sub(1);
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            self.open = false;
            return None;
        }
        let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        let mut action = None;
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .show(ui, |ui| {
                for (index, command_index) in results.into_iter().enumerate() {
                    let command = &registry.commands[command_index];
                    let reason = availability.reason(command.action, command.scope);
                    let color = if reason.is_some() {
                        theme.text.disabled
                    } else {
                        theme.text.primary
                    };
                    let response = ui.add_enabled(
                        reason.is_none(),
                        egui::Button::new(RichText::new(command.label).color(color))
                            .selected(index == self.selected)
                            .min_size(egui::vec2(ui.available_width(), 28.0)),
                    );
                    if let Some(reason) = reason {
                        response.on_hover_text(reason);
                    } else if response.clicked() || (enter && index == self.selected) {
                        action = Some(command.action);
                        self.open = false;
                    }
                }
            });
        action
    }
}
