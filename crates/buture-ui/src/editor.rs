//! Native editor draws only visible lines and maps caret hitboxes through actual font layouts.
use crate::Theme;
use buture_editor::{Buffer, EditorError, FileEvent, Files};
use egui::{Color32, Event, FontId, Key, RichText, Sense, Stroke, Ui, pos2, text::CCursor, vec2};
use std::{
    collections::BTreeSet,
    ops::Range,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}
impl Selection {
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }
    pub fn empty(&self) -> bool {
        self.anchor == self.head
    }
}
#[derive(Default)]
pub struct View {
    pub selection: Selection,
    pub follow_caret: bool,
}
pub struct Document {
    pub buffer: Buffer,
    pub view: View,
    pub pinned: bool,
    pub preview: bool,
}
#[derive(Default)]
pub struct Group {
    pub tabs: Vec<usize>,
    pub active: Option<usize>,
}
pub struct EditorSpace {
    pub documents: Vec<Document>,
    pub active: usize,
    pub groups: Vec<Group>,
    pub group_active: usize,
    pub files: Files,
    pub saving: BTreeSet<PathBuf>,
    pub loading: BTreeSet<PathBuf>,
    preview_requests: BTreeSet<PathBuf>,
    closed: Vec<usize>,
    close_review: Vec<usize>,
    close_after_save: BTreeSet<usize>,
}
impl Default for EditorSpace {
    fn default() -> Self {
        Self {
            documents: Vec::new(),
            active: 0,
            groups: vec![Group::default()],
            group_active: 0,
            files: Files::default(),
            saving: BTreeSet::new(),
            loading: BTreeSet::new(),
            preview_requests: BTreeSet::new(),
            closed: Vec::new(),
            close_review: Vec::new(),
            close_after_save: BTreeSet::new(),
        }
    }
}
#[derive(Debug, Clone, Copy)]
enum TabAction {
    Close(usize),
    Others(usize),
    Right(usize),
    Pin(usize),
    Keep(usize),
    Restore,
}
impl EditorSpace {
    pub fn open_preview(&mut self, root: PathBuf, path: PathBuf) {
        self.preview_requests.insert(path.clone());
        self.open(root, path);
    }
    pub fn open(&mut self, root: PathBuf, path: PathBuf) {
        if let Some(index) = self
            .documents
            .iter()
            .position(|d| d.buffer.path.as_ref() == Some(&path))
            && self.groups.iter().any(|g| g.tabs.contains(&index))
        {
            if !self.groups[self.group_active].tabs.contains(&index) {
                self.groups[self.group_active].tabs.push(index);
            }
            self.preview_requests.remove(&path);
            self.active = index;
            self.groups[self.group_active].active = Some(index);
            return;
        }
        if self.loading.insert(path.clone()) {
            self.files.open(root, path);
        }
    }
    fn close_clean(&mut self, index: usize) {
        let group = &mut self.groups[self.group_active];
        group.tabs.retain(|i| *i != index);
        if group.active == Some(index) {
            group.active = group.tabs.last().copied();
        }
        self.closed.push(index);
        if self.closed.len() > 20 {
            self.closed.remove(0);
        }
        if let Some(active) = group.active {
            self.active = active;
        }
    }
    fn close_tabs(&mut self, indices: Vec<usize>) {
        for index in indices {
            if self.documents[index].buffer.dirty() {
                if !self.close_review.contains(&index) {
                    self.close_review.push(index);
                }
            } else {
                self.close_clean(index);
            }
        }
    }
    fn action(&mut self, action: TabAction) {
        match action {
            TabAction::Close(i) => self.close_tabs(vec![i]),
            TabAction::Others(i) => {
                let indices = self.groups[self.group_active]
                    .tabs
                    .iter()
                    .copied()
                    .filter(|j| *j != i && !self.documents[*j].pinned)
                    .collect();
                self.close_tabs(indices);
            }
            TabAction::Right(i) => {
                let group = &self.groups[self.group_active];
                let start = group
                    .tabs
                    .iter()
                    .position(|j| *j == i)
                    .unwrap_or(group.tabs.len());
                let indices = group
                    .tabs
                    .iter()
                    .skip(start + 1)
                    .copied()
                    .filter(|j| !self.documents[*j].pinned)
                    .collect();
                self.close_tabs(indices);
            }
            TabAction::Pin(i) => {
                self.documents[i].pinned = !self.documents[i].pinned;
                self.documents[i].preview = false;
            }
            TabAction::Keep(i) => self.documents[i].preview = false,
            TabAction::Restore => {
                if let Some(index) = self.closed.pop()
                    && !self.groups[self.group_active].tabs.contains(&index)
                {
                    self.groups[self.group_active].tabs.push(index);
                    self.groups[self.group_active].active = Some(index);
                    self.active = index;
                }
            }
        }
    }
    pub fn poll(&mut self) -> Option<String> {
        let mut message = None;
        while let Ok(event) = self.files.events.try_recv() {
            match event {
                FileEvent::Opened(buffer) => {
                    let preview = buffer
                        .path
                        .as_ref()
                        .is_some_and(|p| self.preview_requests.remove(p));
                    if let Some(path) = &buffer.path {
                        self.loading.remove(path);
                    }
                    if preview {
                        let old = self.groups[self.group_active]
                            .tabs
                            .iter()
                            .copied()
                            .find(|i| {
                                self.documents[*i].preview && !self.documents[*i].buffer.dirty()
                            });
                        if let Some(old) = old {
                            self.close_clean(old);
                        }
                    }
                    let index = self
                        .documents
                        .iter()
                        .position(|d| d.buffer.path == buffer.path)
                        .unwrap_or(self.documents.len());
                    let document = Document {
                        buffer,
                        view: View::default(),
                        pinned: false,
                        preview,
                    };
                    if index == self.documents.len() {
                        self.documents.push(document);
                    } else {
                        self.documents[index] = document;
                    }
                    if !self.groups[self.group_active].tabs.contains(&index) {
                        self.groups[self.group_active].tabs.push(index);
                    }
                    self.active = index;
                    self.groups[self.group_active].active = Some(index);
                }
                FileEvent::Saved(saved) => {
                    self.saving.remove(&saved.path);
                    if let Some(index) = self
                        .documents
                        .iter()
                        .position(|d| d.buffer.path.as_ref() == Some(&saved.path))
                    {
                        self.documents[index].buffer.accept_saved(saved);
                        if self.close_after_save.remove(&index)
                            && !self.documents[index].buffer.dirty()
                        {
                            self.close_clean(index);
                            self.close_review.retain(|i| *i != index);
                        }
                    }
                }
                FileEvent::Failed { path, error } => {
                    self.loading.remove(&path);
                    self.saving.remove(&path);
                    message = Some(error.to_string());
                }
            }
        }
        message
    }
    fn save_document(&mut self, index: usize) {
        if let Some(document) = self.documents.get(index)
            && let Some(path) = &document.buffer.path
            && self.saving.insert(path.clone())
        {
            self.files.save(document.buffer.clone());
        }
    }
    pub fn save_active(&mut self) {
        self.save_document(self.active);
    }
    fn tab_bar(&mut self, ui: &mut Ui) -> bool {
        let mut action = None;
        let mut save = false;
        ui.horizontal_wrapped(|ui| {
            for index in self.groups[self.group_active].tabs.clone() {
                let document = &self.documents[index];
                let name = document
                    .buffer
                    .path
                    .as_ref()
                    .map(|p| {
                        p.file_name()
                            .unwrap_or(p.as_os_str())
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|| "Untitled".into());
                let label = format!(
                    "{}{}{}",
                    if document.pinned { "[pin] " } else { "" },
                    name,
                    if document.buffer.dirty() { " *" } else { "" }
                );
                let text = if document.preview {
                    RichText::new(label).italics()
                } else {
                    RichText::new(label)
                };
                let response =
                    ui.selectable_label(self.groups[self.group_active].active == Some(index), text);
                if response.clicked() {
                    self.active = index;
                    self.groups[self.group_active].active = Some(index);
                }
                if response.double_clicked() {
                    action = Some(TabAction::Keep(index));
                }
                response.context_menu(|ui| {
                    for (label, event) in [
                        ("Close", TabAction::Close(index)),
                        ("Close others", TabAction::Others(index)),
                        ("Close to right", TabAction::Right(index)),
                        ("Pin / unpin", TabAction::Pin(index)),
                        ("Keep open", TabAction::Keep(index)),
                    ] {
                        if ui.button(label).clicked() {
                            action = Some(event);
                            ui.close();
                        }
                    }
                });
                if ui.small_button("x").clicked() {
                    action = Some(TabAction::Close(index));
                }
            }
            if ui.small_button("Restore closed").clicked() {
                action = Some(TabAction::Restore);
            }
            if ui.button("Save").clicked() {
                save = true;
            }
        });
        if let Some(action) = action {
            self.action(action);
        }
        save
    }
    fn close_dialog(&mut self, ctx: &egui::Context) {
        if let Some(index) = self.close_review.first().copied() {
            let name = self.documents[index]
                .buffer
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Untitled".into());
            egui::Window::new("Unsaved changes").collapsible(false).show(ctx,|ui|{
        ui.label(name);ui.label("Save this document or confirm discarding its unsaved changes before closing.");
        if ui.button("Save and close").clicked(){self.close_after_save.insert(index);self.save_document(index);}
        if ui.button("Discard changes and close").clicked(){self.documents[index].buffer.revert_to_saved();self.close_clean(index);self.close_review.remove(0);}
        if ui.button("Cancel").clicked(){self.close_review.clear();self.close_after_save.clear();}
    });
        }
    }
    pub fn show(&mut self, ui: &mut Ui, theme: &Theme) -> Option<String> {
        self.close_dialog(ui.ctx());
        let mut save = self.tab_bar(ui);
        ui.separator();
        let Some(active) = self.groups[self.group_active].active else {
            if !self.loading.is_empty() {
                ui.spinner();
            } else {
                ui.label("No files open");
            }
            return None;
        };
        self.active = active;
        let document = &mut self.documents[active];
        if document.buffer.dirty() {
            document.preview = false;
        }
        let mut error = None;
        match render(ui, theme, &mut document.buffer, &mut document.view) {
            Ok(request) => save |= request,
            Err(e) => error = Some(e.to_string()),
        }
        if save {
            self.save_active();
        }
        error
    }
    pub fn active_path(&self) -> Option<&Path> {
        self.documents.get(self.active)?.buffer.path.as_deref()
    }
}
fn move_caret(buffer: &Buffer, selection: &mut Selection, key: Key, shift: bool, word: bool) {
    let len = buffer.len_chars();
    let head = selection.head.min(len);
    let (row, column) = buffer.position(head).unwrap_or((0, 0));
    let target = match key {
        Key::ArrowLeft if !shift && !selection.empty() => selection.range().start,
        Key::ArrowRight if !shift && !selection.empty() => selection.range().end,
        Key::ArrowLeft => {
            let mut p = head.saturating_sub(1);
            if word {
                while p > 0 && buffer.rope().char(p).is_whitespace() {
                    p -= 1;
                }
                while p > 0 && is_word(buffer.rope().char(p - 1)) {
                    p -= 1;
                }
            } else if p > 0
                && buffer.rope().get_char(p) == Some('\n')
                && buffer.rope().get_char(p - 1) == Some('\r')
            {
                p -= 1;
            }
            p
        }
        Key::ArrowRight => {
            let mut p = (head + 1).min(len);
            if word {
                while p < len && is_word(buffer.rope().char(p)) {
                    p += 1;
                }
                while p < len && buffer.rope().char(p).is_whitespace() {
                    p += 1;
                }
            } else if buffer.rope().get_char(head) == Some('\r')
                && buffer.rope().get_char(p) == Some('\n')
            {
                p = (p + 1).min(len);
            }
            p
        }
        Key::ArrowUp | Key::ArrowDown => {
            let row = if key == Key::ArrowUp {
                row.saturating_sub(1)
            } else {
                (row + 1).min(buffer.line_count() - 1)
            };
            let length = buffer
                .line(row)
                .unwrap_or_default()
                .trim_end_matches(['\r', '\n'])
                .chars()
                .count();
            buffer.offset(row, column.min(length)).unwrap_or(head)
        }
        Key::Home => {
            if word {
                0
            } else {
                buffer.line_start(row).unwrap_or(0)
            }
        }
        Key::End => {
            if word {
                len
            } else {
                buffer
                    .offset(
                        row,
                        buffer
                            .line(row)
                            .unwrap_or_default()
                            .trim_end_matches(['\r', '\n'])
                            .chars()
                            .count(),
                    )
                    .unwrap_or(len)
            }
        }
        _ => head,
    };
    selection.head = target;
    if !shift {
        selection.anchor = target;
    }
}
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
fn insert(buffer: &mut Buffer, selection: &mut Selection, text: &str) -> Result<(), EditorError> {
    let range = selection.range();
    let end = range.start + text.chars().count();
    buffer.replace(range, text)?;
    selection.head = end;
    selection.anchor = end;
    Ok(())
}
fn input(ui: &mut Ui, buffer: &mut Buffer, view: &mut View) -> Result<bool, EditorError> {
    let mut save = false;
    let events = ui.input(|i| i.events.clone());
    for event in events {
        match event {
            Event::Text(text) | Event::Ime(egui::ImeEvent::Commit(text)) => {
                insert(buffer, &mut view.selection, &text)?
            }
            Event::Paste(text) => {
                let text = text
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
                    .replace('\n', buffer.line_ending.text());
                insert(buffer, &mut view.selection, &text)?;
            }
            Event::Copy => {
                ui.ctx().copy_text(buffer.slice(view.selection.range())?);
            }
            Event::Cut => {
                ui.ctx().copy_text(buffer.slice(view.selection.range())?);
                insert(buffer, &mut view.selection, "")?;
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if modifiers.command {
                    match key {
                        Key::S => save = true,
                        Key::A => {
                            view.selection.anchor = 0;
                            view.selection.head = buffer.len_chars();
                        }
                        Key::Z => {
                            if modifiers.shift {
                                buffer.redo();
                            } else {
                                buffer.undo();
                            }
                            view.selection.head = view.selection.head.min(buffer.len_chars());
                            view.selection.anchor = view.selection.head;
                        }
                        Key::Y => {
                            buffer.redo();
                            view.selection.head = view.selection.head.min(buffer.len_chars());
                            view.selection.anchor = view.selection.head;
                        }
                        _ => {
                            move_caret(buffer, &mut view.selection, key, modifiers.shift, true);
                        }
                    }
                } else {
                    match key {
                        Key::Enter => {
                            insert(buffer, &mut view.selection, buffer.line_ending.text())?;
                        }
                        Key::Tab => insert(buffer, &mut view.selection, "    ")?,
                        Key::Backspace | Key::Delete => {
                            if view.selection.empty() {
                                let head = view.selection.head;
                                if key == Key::Backspace {
                                    move_caret(
                                        buffer,
                                        &mut view.selection,
                                        Key::ArrowLeft,
                                        true,
                                        false,
                                    );
                                } else {
                                    move_caret(
                                        buffer,
                                        &mut view.selection,
                                        Key::ArrowRight,
                                        true,
                                        false,
                                    );
                                }
                                view.selection.anchor = head;
                            }
                            insert(buffer, &mut view.selection, "")?;
                        }
                        _ => move_caret(buffer, &mut view.selection, key, modifiers.shift, false),
                    }
                }
                view.follow_caret = true;
            }
            _ => {}
        }
    }
    Ok(save)
}
fn expand_tabs(source: &str) -> (String, Vec<usize>) {
    let mut text = String::new();
    let mut map = vec![0];
    let mut col = 0;
    for (index, c) in source.chars().enumerate() {
        if c == '\t' {
            let spaces = 4 - col % 4;
            for n in 0..spaces {
                text.push(' ');
                map.push(if n + 1 == spaces { index + 1 } else { index });
            }
            col += spaces;
        } else {
            text.push(c);
            map.push(index + 1);
            col += 1;
        }
    }
    (text, map)
}
pub fn render(
    ui: &mut Ui,
    theme: &Theme,
    buffer: &mut Buffer,
    view: &mut View,
) -> Result<bool, EditorError> {
    let id = ui.id().with("code-editor");
    ui.interact(ui.max_rect(), id, Sense::focusable_noninteractive());
    view.selection.head = view.selection.head.min(buffer.len_chars());
    view.selection.anchor = view.selection.anchor.min(buffer.len_chars());
    let focused = ui.memory(|m| m.has_focus(id));
    let save = if focused {
        input(ui, buffer, view)?
    } else {
        false
    };
    let font = FontId::monospace(theme.typography.code);
    let row_height = theme.typography.code + 6.0;
    let gutter = 60.0;
    let caret_row = buffer.char_to_line(view.selection.head)?;
    let char_width = ui.fonts_mut(|f| f.glyph_width(&font, 'M'));
    let mut scroll = egui::ScrollArea::both()
        .id_salt("code-scroll")
        .auto_shrink([false, false]);
    if view.follow_caret {
        scroll = scroll.vertical_scroll_offset(
            (caret_row as f32 * row_height - ui.available_height() * 0.4).max(0.0),
        );
        view.follow_caret = false;
    }
    scroll.show_rows(ui, row_height, buffer.line_count(), |ui, range| {
        ui.set_min_width(
            (buffer.width_hint.min(100_000) as f32 * char_width + gutter + 20.0)
                .max(ui.available_width()),
        );
        for row in range {
            let source = buffer.line(row).unwrap_or_default();
            let source = source.trim_end_matches(['\r', '\n']);
            let (display, map) = expand_tabs(source);
            let galley =
                ui.painter()
                    .layout_job(crate::syntax::layout(&display, font.clone(), theme));
            let (rect, response) = ui.allocate_exact_size(
                vec2(ui.available_width(), row_height),
                Sense::click_and_drag(),
            );
            let text_pos = pos2(rect.left() + gutter, rect.top() + 2.0);
            let start = buffer.line_start(row).unwrap_or(0);
            if (response.clicked() || response.dragged())
                && let Some(pointer) = response.interact_pointer_pos()
            {
                let display_col = galley
                    .cursor_from_pos(pointer - text_pos)
                    .index
                    .min(map.len() - 1);
                let offset = start + map[display_col];
                if response.clicked() && !ui.input(|i| i.modifiers.shift) {
                    view.selection.anchor = offset;
                }
                view.selection.head = offset;
                ui.memory_mut(|m| m.request_focus(id));
            }
            if row == caret_row {
                ui.painter()
                    .rect_filled(rect, 0, Color32::from(theme.surface.panel));
            }
            ui.painter().text(
                pos2(rect.left() + gutter - 12.0, rect.top() + 2.0),
                egui::Align2::RIGHT_TOP,
                (row + 1).to_string(),
                font.clone(),
                theme.text.muted.into(),
            );
            let selected = view.selection.range();
            let end = start + source.chars().count();
            if selected.start <= end && selected.end > start {
                let a = selected.start.saturating_sub(start).min(end - start);
                let b = selected.end.saturating_sub(start).min(end - start);
                let da = map.iter().position(|p| *p == a).unwrap_or(0);
                let db = map.iter().rposition(|p| *p == b).unwrap_or(map.len() - 1);
                let x1 = galley.pos_from_cursor(CCursor::new(da)).left();
                let x2 = galley.pos_from_cursor(CCursor::new(db)).left();
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        pos2(text_pos.x + x1, rect.top()),
                        pos2(text_pos.x + x2.max(x1 + char_width), rect.bottom()),
                    ),
                    0,
                    Color32::from(theme.accent.emerald_muted),
                );
            }
            if focused && row == caret_row && ui.input(|i| i.time % 1.0 < 0.65) {
                let col = view.selection.head - start;
                let display_col = map.iter().rposition(|p| *p == col).unwrap_or(map.len() - 1);
                let x = galley.pos_from_cursor(CCursor::new(display_col)).left() + text_pos.x;
                ui.painter().line_segment(
                    [pos2(x, rect.top() + 2.0), pos2(x, rect.bottom() - 2.0)],
                    Stroke::new(1.5_f32, Color32::from(theme.accent.emerald)),
                );
            }
            ui.painter()
                .galley(text_pos, galley, theme.text.primary.into());
        }
    });
    let (row, col) = buffer.position(view.selection.head)?;
    ui.label(
        RichText::new(format!(
            "Ln {}, Col {} · UTF-8 · {:?}",
            row + 1,
            col + 1,
            buffer.line_ending
        ))
        .small(),
    );
    if focused {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(300));
    }
    Ok(save)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_edits_utf8_without_rebuilding_a_whole_document() {
        let ctx = egui::Context::default();
        let theme = Theme::dark();
        let mut buffer = Buffer::from_text("abc\r\n");
        let mut view = View::default();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(900.0, 600.0),
            )),
            events: vec![
                Event::Text("á🦀".into()),
                Event::Key {
                    key: Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.memory_mut(|m| m.request_focus(ui.id().with("code-editor")));
                render(ui, &theme, &mut buffer, &mut view).expect("editor");
            });
        });
        assert_eq!(buffer.rope().to_string(), "á🦀\r\nabc\r\n");
        assert_eq!(buffer.position(view.selection.head).expect("caret"), (1, 0));
        assert!(buffer.dirty());
    }
    #[test]
    fn large_documents_generate_only_viewport_shapes() {
        let ctx = egui::Context::default();
        let theme = Theme::dark();
        let mut buffer = Buffer::from_text(&"line\n".repeat(100_000));
        let mut view = View::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(900.0, 600.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    render(ui, &theme, &mut buffer, &mut view).expect("render");
                });
            },
        );
        assert!(output.shapes.len() < 500);
        assert_eq!(buffer.revision, 0);
    }
    #[test]
    fn closing_tabs_preserves_dirty_and_pinned_documents() {
        let mut space = EditorSpace::default();
        for i in 0..3 {
            let mut buffer = Buffer::from_text("saved");
            buffer.path = Some(format!("/file-{i}").into());
            space.documents.push(Document {
                buffer,
                view: View::default(),
                pinned: i == 1,
                preview: false,
            });
        }
        space.documents[2]
            .buffer
            .insert(0, "modified ")
            .expect("edit");
        space.groups[0].tabs = vec![0, 1, 2];
        space.groups[0].active = Some(0);
        space.action(TabAction::Others(0));
        assert_eq!(space.groups[0].tabs, vec![0, 1, 2]);
        assert_eq!(space.close_review, vec![2]);
        space.action(TabAction::Close(0));
        assert!(!space.groups[0].tabs.contains(&0));
        space.action(TabAction::Restore);
        assert!(space.groups[0].tabs.contains(&0));
        assert!(space.documents[2].buffer.dirty());
    }
}
