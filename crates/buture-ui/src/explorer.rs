//! Dense, keyboard accessible workspace tree. Nodes are supplied by the workspace domain.
use egui::{Key, Ui};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct TreeRow {
    pub path: PathBuf,
    pub directory: bool,
    pub depth: usize,
    pub expanded: bool,
}
#[derive(Default)]
pub struct ExplorerView {
    pub rows: Vec<TreeRow>,
    pub selected: Option<PathBuf>,
    focused: Option<egui::Id>,
}
#[derive(Debug)]
pub enum TreeAction {
    Toggle(PathBuf),
    Open(PathBuf),
    Menu(PathBuf),
}
impl ExplorerView {
    pub fn show(&mut self, ui: &mut Ui) -> Option<TreeAction> {
        let mut action = None;
        let selected_index = self
            .rows
            .iter()
            .position(|r| Some(&r.path) == self.selected.as_ref());
        if self
            .focused
            .is_some_and(|id| ui.memory(|m| m.has_focus(id)))
        {
            let delta = ui.input(|i| {
                if i.key_pressed(Key::ArrowDown) {
                    1
                } else if i.key_pressed(Key::ArrowUp) {
                    -1
                } else {
                    0
                }
            });
            if delta != 0 && !self.rows.is_empty() {
                let index = (selected_index.unwrap_or(0) as isize + delta)
                    .clamp(0, self.rows.len() as isize - 1) as usize;
                self.selected = Some(self.rows[index].path.clone());
            }
            if let Some(index) = selected_index {
                let row = &self.rows[index];
                if ui.input(|i| {
                    i.key_pressed(Key::Enter)
                        || (row.directory && !row.expanded && i.key_pressed(Key::ArrowRight))
                        || (row.expanded && i.key_pressed(Key::ArrowLeft))
                }) {
                    action = Some(if row.directory {
                        TreeAction::Toggle(row.path.clone())
                    } else {
                        TreeAction::Open(row.path.clone())
                    });
                }
            }
        }
        egui::ScrollArea::vertical()
            .id_salt("workspace-tree")
            .show_rows(ui, 24.0, self.rows.len(), |ui, range| {
                for index in range {
                    let row = &self.rows[index];
                    ui.horizontal(|ui| {
                        ui.add_space(row.depth as f32 * 12.0);
                        let prefix = if row.directory {
                            if row.expanded { "-" } else { "+" }
                        } else {
                            "·"
                        };
                        let name = row
                            .path
                            .file_name()
                            .unwrap_or(row.path.as_os_str())
                            .to_string_lossy();
                        let response = ui.selectable_label(
                            self.selected.as_ref() == Some(&row.path),
                            format!("{prefix} {name}"),
                        );
                        if response.clicked() {
                            response.request_focus();
                            self.focused = Some(response.id);
                            self.selected = Some(row.path.clone());
                        }
                        if response.has_focus() {
                            self.focused = Some(response.id);
                        }
                        if response.secondary_clicked() {
                            self.selected = Some(row.path.clone());
                        }
                        if response.double_clicked() || (row.directory && response.clicked()) {
                            action = Some(if row.directory {
                                TreeAction::Toggle(row.path.clone())
                            } else {
                                TreeAction::Open(row.path.clone())
                            });
                        }
                        response.context_menu(|ui| {
                            if ui.button("File actions…").clicked() {
                                action = Some(TreeAction::Menu(row.path.clone()));
                                ui.close();
                            }
                        });
                    });
                }
            });
        action
    }
}
