//! Quick Open shares floating surface and keyboard behavior with the command picker.
use crate::Theme;
use egui::{Key, RichText, Ui};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
pub struct QuickOpen {
    pub open: bool,
    query: String,
    focus: bool,
    selected: usize,
    index: buture_search::index::Index,
    root: Option<PathBuf>,
    built: bool,
    building: bool,
    generation: u64,
    requested: Option<u64>,
    returned: Option<u64>,
    files: Vec<buture_search::index::File>,
    count: usize,
    partial: bool,
    error: Option<String>,
    changed: Instant,
}
impl Default for QuickOpen {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            focus: false,
            selected: 0,
            index: Default::default(),
            root: None,
            built: false,
            building: false,
            generation: 0,
            requested: None,
            returned: None,
            files: Vec::new(),
            count: 0,
            partial: false,
            error: None,
            changed: Instant::now(),
        }
    }
}
impl QuickOpen {
    pub fn busy(&self) -> bool {
        self.open && (self.building || (self.built && self.returned != Some(self.generation)))
    }
    pub fn activate(&mut self, root: Option<PathBuf>) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.focus = true;
        self.error = None;
        self.changed = Instant::now();
        self.generation += 1;
        self.requested = None;
        self.returned = None;
        if self.root != root {
            self.root = root;
            self.built = false;
            self.building = false;
            self.files.clear();
            self.count = 0;
            self.partial = false;
        }
        if !self.built
            && let Some(root) = &self.root
        {
            self.building = self.index.build(root.clone());
        }
    }
    pub fn set_query(&mut self, query: &str) {
        self.query = query.into();
        self.files.clear();
        self.generation += 1;
        self.requested = None;
        self.returned = None;
        self.changed = Instant::now();
    }
    pub fn show(&mut self, ui: &mut Ui, theme: &Theme) -> Option<PathBuf> {
        while let Ok(event) = self.index.events.try_recv() {
            match event {
                buture_search::index::Event::Built {
                    root,
                    files,
                    partial,
                } if self.root.as_ref() == Some(&root) => {
                    self.count = files;
                    self.partial = partial;
                    self.built = true;
                    self.building = false;
                    self.requested = None;
                    self.returned = None;
                }
                buture_search::index::Event::Results {
                    root,
                    generation,
                    files,
                } if self.root.as_ref() == Some(&root) && generation == self.generation => {
                    self.files = files
                }
                buture_search::index::Event::Failed { root, message }
                    if self.root.as_ref() == Some(&root) =>
                {
                    self.error = Some(message);
                    self.building = false;
                }
                _ => {}
            }
        }
        ui.set_min_width(560.0);
        ui.label(RichText::new("Quick Open").strong());
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text("Search filenames and paths…")
                .desired_width(ui.available_width()),
        );
        if self.focus {
            response.request_focus();
            self.focus = false;
        }
        if response.changed() {
            self.files.clear();
            self.generation += 1;
            self.requested = None;
            self.returned = None;
            self.selected = 0;
            self.changed = Instant::now();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            self.open = false;
            return None;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown)) {
            self.selected = (self.selected + 1).min(self.files.len().saturating_sub(1));
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp)) {
            self.selected = self.selected.saturating_sub(1);
        }
        if self.built
            && self.requested != Some(self.generation)
            && self.changed.elapsed() > Duration::from_millis(100)
            && let Some(root) = &self.root
            && self
                .index
                .query(root.clone(), self.query.clone(), self.generation)
        {
            self.requested = Some(self.generation);
        }
        let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        let mut selected = None;
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} indexed files{}",
                self.count,
                if self.partial {
                    " · partial index"
                } else {
                    ""
                }
            ));
            if ui.small_button("Refresh").clicked()
                && let Some(root) = &self.root
            {
                self.built = false;
                self.building = self.index.build(root.clone());
                self.files.clear();
            }
            if self.building {
                ui.spinner();
            }
        });
        if let Some(error) = &self.error {
            ui.label(error);
        }
        if self.root.is_none() {
            ui.label("Open a folder first");
        }
        self.selected = self.selected.min(self.files.len().saturating_sub(1));
        egui::ScrollArea::vertical().max_height(360.0).show_rows(
            ui,
            42.0,
            self.files.len(),
            |ui, range| {
                for index in range {
                    let file = &self.files[index];
                    let response = ui.selectable_label(
                        index == self.selected,
                        RichText::new(&file.name).color(theme.text.primary),
                    );
                    ui.label(
                        RichText::new(file.relative.display().to_string())
                            .small()
                            .color(theme.text.muted),
                    );
                    if response.clicked() || (enter && index == self.selected) {
                        selected = Some(file.path.clone());
                        self.open = false;
                    }
                }
            },
        );
        selected
    }
}
