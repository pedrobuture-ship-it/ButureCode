//! Native editor draws only visible lines and maps caret hitboxes through actual font layouts.
use crate::Theme;
use buture_editor::{Buffer, EditorError, FileEvent, Files};
use egui::{Color32, Event, FontId, Key, RichText, Sense, Stroke, Ui, pos2, text::CCursor, vec2};
use std::{
    collections::{BTreeMap, BTreeSet},
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
type BracketCache = ((u64, usize, bool), Option<(usize, usize)>);
#[derive(Default)]
pub struct View {
    pub selection: Selection,
    pub additional: Vec<Selection>,
    folded: BTreeSet<usize>,
    fold_revision: u64,
    bracket_cache: Option<BracketCache>,
    focus: Option<egui::Id>,
    pub follow_caret: bool,
    scroll: egui::Vec2,
    dragging: bool,
}
pub struct Document {
    pub buffer: Buffer,
    pub view: View,
    pub pinned: bool,
    pub preview: bool,
    pub parsed: Option<std::sync::Arc<buture_editor::parsing::Parsed>>,
    parse_requested: Option<(u64, u64)>,
}
#[derive(Default)]
pub struct Group {
    views: BTreeMap<usize, View>,
    pub tabs: Vec<usize>,
    pub active: Option<usize>,
}
pub struct EditorSpace {
    pub documents: Vec<Document>,
    pub active: usize,
    pub groups: Vec<Group>,
    pub group_active: usize,
    pub files: Files,
    parsing: buture_editor::parsing::Service,
    pub saving: BTreeSet<PathBuf>,
    pub loading: BTreeSet<PathBuf>,
    preview_requests: BTreeSet<PathBuf>,
    closed: Vec<usize>,
    close_review: Vec<usize>,
    close_after_save: BTreeMap<usize, usize>,
    orientation: buture_editor::session::Orientation,
    session: Option<buture_editor::session::Store>,
    workspace: Option<PathBuf>,
    session_loaded: bool,
    opening_groups: BTreeMap<PathBuf, Vec<(usize, bool)>>,
    restored_active: BTreeMap<usize, PathBuf>,
    close_owners: BTreeMap<usize, usize>,
}
impl Default for EditorSpace {
    fn default() -> Self {
        Self {
            documents: Vec::new(),
            active: 0,
            groups: vec![Group::default()],
            group_active: 0,
            files: Files::default(),
            parsing: Default::default(),
            saving: BTreeSet::new(),
            loading: BTreeSet::new(),
            preview_requests: BTreeSet::new(),
            closed: Vec::new(),
            close_review: Vec::new(),
            close_after_save: BTreeMap::new(),
            orientation: Default::default(),
            session: None,
            workspace: None,
            session_loaded: false,
            opening_groups: BTreeMap::new(),
            restored_active: BTreeMap::new(),
            close_owners: BTreeMap::new(),
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
    Move(usize),
}
impl EditorSpace {
    pub fn configure_session(&mut self, path: PathBuf) {
        self.session = Some(buture_editor::session::Store::start(path));
    }
    pub fn set_workspace(&mut self, root: PathBuf) {
        self.persist();
        self.workspace = Some(root.clone());
        self.session_loaded = false;
        if let Some(store) = &self.session {
            store.load(root);
        } else {
            self.session_loaded = true;
        }
    }
    pub fn persist(&self) {
        if !self.session_loaded || !self.loading.is_empty() {
            return;
        }
        if let (Some(store), Some(root)) = (&self.session, &self.workspace) {
            let groups = self
                .groups
                .iter()
                .map(|g| buture_editor::session::Group {
                    tabs: g
                        .tabs
                        .iter()
                        .filter_map(|i| {
                            let document = &self.documents[*i];
                            document
                                .buffer
                                .path
                                .as_ref()
                                .filter(|p| p.starts_with(root))
                                .map(|p| buture_editor::session::Tab {
                                    path: p.clone(),
                                    pinned: document.pinned,
                                })
                        })
                        .collect(),
                    active: g.active.and_then(|i| self.documents[i].buffer.path.clone()),
                })
                .collect();
            store.save(buture_editor::session::Layout {
                workspace: root.clone(),
                orientation: self.orientation,
                groups,
            });
        }
    }
    pub fn flush_session(&self) {
        self.persist();
        if let Some(store) = &self.session {
            store.flush();
        }
    }
    pub fn split(&mut self, orientation: buture_editor::session::Orientation) {
        if self.groups.len() >= 4 {
            return;
        }
        let active = self.groups[self.group_active].active;
        let mut group = Group {
            active,
            ..Default::default()
        };
        if let Some(index) = active {
            group.tabs.push(index);
        }
        self.groups.push(group);
        self.orientation = orientation;
        self.group_active = self.groups.len() - 1;
        self.persist();
    }
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
        self.opening_groups
            .entry(path.clone())
            .or_default()
            .push((self.group_active, false));
        if self.loading.insert(path.clone()) {
            self.files.open(root, path);
        }
    }
    fn close_clean_in(&mut self, index: usize, owner: usize) {
        let previous = self.group_active;
        self.group_active = owner;
        self.close_clean(index);
        self.group_active = previous.min(self.groups.len() - 1);
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
                    self.close_owners.insert(index, self.group_active);
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
            TabAction::Move(index) => {
                let from = self.group_active;
                let target = (from + 1) % self.groups.len();
                if from != target {
                    self.groups[from].tabs.retain(|i| *i != index);
                    if self.groups[from].active == Some(index) {
                        self.groups[from].active = self.groups[from].tabs.last().copied();
                    }
                    if !self.groups[target].tabs.contains(&index) {
                        self.groups[target].tabs.push(index);
                    }
                    self.groups[target].active = Some(index);
                    self.group_active = target;
                    self.active = index;
                }
            }
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
        if let Some(store) = &self.session
            && let Ok(event) = store.events.try_recv()
        {
            match event {
                buture_editor::session::Event::Failed(error) => {
                    self.session_loaded = false;
                    return Some(error.to_string());
                }
                buture_editor::session::Event::Loaded(layout) => {
                    self.session_loaded = true;
                    if let Some(layout) = layout
                        && self.documents.iter().all(|d| !d.buffer.dirty())
                    {
                        self.groups = (0..layout.groups.len()).map(|_| Group::default()).collect();
                        self.group_active = 0;
                        self.orientation = layout.orientation;
                        for (index, group) in layout.groups.into_iter().enumerate() {
                            if let Some(active) = group.active {
                                self.restored_active.insert(index, active);
                            }
                            for tab in group.tabs {
                                self.opening_groups
                                    .entry(tab.path.clone())
                                    .or_default()
                                    .push((index, tab.pinned));
                                if self.loading.insert(tab.path.clone()) {
                                    self.files.open(layout.workspace.clone(), tab.path);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut message = None;
        while let Ok(event) = self.parsing.events.try_recv() {
            if let Some(document) = self
                .documents
                .iter_mut()
                .find(|d| d.buffer.path.as_ref() == Some(&event.path))
            {
                match event.result {
                    Ok(parsed)
                        if parsed.revision == document.buffer.revision
                            && parsed.content_id == document.buffer.content_id() =>
                    {
                        document.parsed = Some(std::sync::Arc::new(parsed))
                    }
                    Ok(_) => {}
                    Err(error) => message = Some(format!("Parser: {error}")),
                }
            }
        }
        for document in &mut self.documents {
            let key = (document.buffer.revision, document.buffer.content_id());
            if document.parse_requested != Some(key)
                && let Some(path) = &document.buffer.path
                && buture_editor::parsing::LanguageKind::from_path(path).is_some()
                && self
                    .parsing
                    .request(path.clone(), document.buffer.rope().clone(), key.0, key.1)
            {
                document.parse_requested = Some(key);
            }
        }
        while let Ok(event) = self.files.events.try_recv() {
            match event {
                FileEvent::Opened(buffer) => {
                    let preview = buffer
                        .path
                        .as_ref()
                        .is_some_and(|p| self.preview_requests.remove(p));
                    let targets = buffer
                        .path
                        .as_ref()
                        .and_then(|p| self.opening_groups.remove(p))
                        .unwrap_or_else(|| vec![(self.group_active, false)]);
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
                        parsed: None,
                        parse_requested: None,
                    };
                    if index == self.documents.len() {
                        self.documents.push(document);
                    } else {
                        self.documents[index] = document;
                    }
                    for (group, pinned) in targets {
                        if group >= self.groups.len() {
                            continue;
                        }
                        self.documents[index].pinned |= pinned;
                        if !self.groups[group].tabs.contains(&index) {
                            self.groups[group].tabs.push(index);
                        }
                        let preferred = self.restored_active.get(&group);
                        if preferred.is_none()
                            || self.groups[group].active.is_none()
                            || preferred == self.documents[index].buffer.path.as_ref()
                        {
                            self.groups[group].active = Some(index);
                        }
                    }
                    self.active = index;
                    self.persist();
                }
                FileEvent::Saved(saved) => {
                    self.saving.remove(&saved.path);
                    if let Some(index) = self
                        .documents
                        .iter()
                        .position(|d| d.buffer.path.as_ref() == Some(&saved.path))
                    {
                        self.documents[index].buffer.accept_saved(saved);
                        if let Some(group) = self.close_after_save.remove(&index)
                            && !self.documents[index].buffer.dirty()
                        {
                            self.close_clean_in(index, group);
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
    fn tab_bar(&mut self, ui: &mut Ui, owner: usize) -> bool {
        let mut action = None;
        let mut save = false;
        ui.horizontal_wrapped(|ui| {
            for index in self.groups[owner].tabs.clone() {
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
                let response = ui.selectable_label(self.groups[owner].active == Some(index), text);
                if response.clicked() {
                    self.group_active = owner;
                    self.active = index;
                    self.groups[owner].active = Some(index);
                }
                if response.double_clicked() {
                    action = Some(TabAction::Keep(index));
                }
                response.context_menu(|ui| {
                    if self.groups.len() > 1 && ui.button("Move to next group").clicked() {
                        action = Some(TabAction::Move(index));
                        ui.close();
                    }
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
            self.group_active = owner;
            self.action(action);
            self.persist();
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
        if ui.button("Save and close").clicked(){self.close_after_save.insert(index,self.group_active);self.save_document(index);}
        if ui.button("Discard changes and close").clicked(){self.documents[index].buffer.revert_to_saved();self.close_clean(index);self.close_review.remove(0);}
        if ui.button("Cancel").clicked(){self.close_review.clear();self.close_after_save.clear();}
    });
        }
    }
    fn show_group(&mut self, ui: &mut Ui, theme: &Theme, owner: usize) -> Option<String> {
        let mut save = self.tab_bar(ui, owner);
        ui.separator();
        let Some(active) = self.groups[owner].active else {
            if !self.loading.is_empty() {
                ui.spinner();
            } else {
                ui.label("No files open");
            }
            return None;
        };
        if ui.rect_contains_pointer(ui.max_rect()) && ui.input(|i| i.pointer.any_pressed()) {
            self.group_active = owner;
            self.active = active;
        }
        let document = &mut self.documents[active];
        if document.buffer.dirty() {
            document.preview = false;
        }
        let view = self.groups[owner].views.entry(active).or_default();
        let mut error = None;
        let parsed = document.parsed.as_deref().filter(|p| {
            p.revision == document.buffer.revision && p.content_id == document.buffer.content_id()
        });
        match render_parsed(ui, theme, &mut document.buffer, view, parsed) {
            Ok(request) => save |= request,
            Err(e) => error = Some(e.to_string()),
        }
        if save {
            self.save_document(active);
        }
        error
    }
    pub fn show(&mut self, ui: &mut Ui, theme: &Theme) -> Option<String> {
        self.close_dialog(ui.ctx());
        let mut error = None;
        ui.horizontal(|ui| {
            if ui.small_button("Split right").clicked() {
                self.split(buture_editor::session::Orientation::Horizontal);
            }
            if ui.small_button("Split down").clicked() {
                self.split(buture_editor::session::Orientation::Vertical);
            }
            if self.groups.len() > 1 && ui.small_button("Merge groups").clicked() {
                let mut tabs = Vec::new();
                for group in &self.groups {
                    for index in &group.tabs {
                        if !tabs.contains(index) {
                            tabs.push(*index);
                        }
                    }
                }
                let active = Some(self.active).filter(|i| tabs.contains(i));
                self.groups = vec![Group {
                    tabs,
                    active,
                    ..Default::default()
                }];
                self.group_active = 0;
                self.persist();
            }
        });
        let mut command = None;
        ui.menu_button("Edit", |ui| {
            for (label, c) in [
                ("Duplicate line", EditCommand::Duplicate),
                ("Move lines up", EditCommand::MoveUp),
                ("Move lines down", EditCommand::MoveDown),
                ("Comment / uncomment", EditCommand::Comment),
                ("Indent", EditCommand::Indent),
                ("Outdent", EditCommand::Outdent),
                ("Select next occurrence", EditCommand::Occurrence),
                ("Add cursor above", EditCommand::CursorUp),
                ("Add cursor below", EditCommand::CursorDown),
                ("Clear extra cursors", EditCommand::ClearCursors),
                ("Toggle fold at cursor", EditCommand::Fold),
            ] {
                if ui.button(label).clicked() {
                    command = Some(c);
                    ui.close();
                }
            }
        });
        if let Some(command) = command
            && let Some(e) = self.perform(command)
        {
            error = Some(e);
        }
        let count = self.groups.len();
        match self.orientation {
            buture_editor::session::Orientation::Horizontal => ui.columns(count, |columns| {
                for (index, column) in columns.iter_mut().enumerate() {
                    column.set_clip_rect(column.clip_rect().intersect(column.max_rect()));
                    column.push_id(index, |ui| {
                        if let Some(e) = self.show_group(ui, theme, index) {
                            error = Some(e);
                        }
                    });
                }
            }),
            buture_editor::session::Orientation::Vertical => {
                let height = (ui.available_height() / count as f32 - 8.0).max(100.0);
                for index in 0..count {
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(("editor-group", index))
                            .max_rect(rect),
                    );
                    child.set_clip_rect(rect);
                    if let Some(e) = self.show_group(&mut child, theme, index) {
                        error = Some(e);
                    }
                }
            }
        }
        error
    }
    pub fn perform(&mut self, command: EditCommand) -> Option<String> {
        let index = self.groups[self.group_active].active?;
        self.active = index;
        let document = &mut self.documents[index];
        let view = self.groups[self.group_active]
            .views
            .entry(index)
            .or_default();
        let parsed = document.parsed.as_deref().filter(|p| {
            p.revision == document.buffer.revision && p.content_id == document.buffer.content_id()
        });
        execute(&mut document.buffer, view, command, parsed)
            .err()
            .map(|e| e.to_string())
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
fn ranges(view: &View) -> Vec<Range<usize>> {
    buture_editor::operations::normalized(
        &std::iter::once(view.selection.range())
            .chain(view.additional.iter().map(Selection::range))
            .collect::<Vec<_>>(),
    )
}
fn assign(view: &mut View, ranges: Vec<Range<usize>>) {
    let mut ranges = ranges.into_iter();
    if let Some(r) = ranges.next() {
        view.selection = Selection {
            anchor: r.start,
            head: r.end,
        };
    }
    view.additional = ranges
        .map(|r| Selection {
            anchor: r.start,
            head: r.end,
        })
        .collect();
    view.follow_caret = true;
}
#[derive(Debug, Clone, Copy)]
pub enum EditCommand {
    Duplicate,
    MoveUp,
    MoveDown,
    Comment,
    Indent,
    Outdent,
    Occurrence,
    CursorUp,
    CursorDown,
    ClearCursors,
    Fold,
    Undo,
    Redo,
    SelectAll,
}
pub fn execute(
    buffer: &mut Buffer,
    view: &mut View,
    command: EditCommand,
    parsed: Option<&buture_editor::parsing::Parsed>,
) -> Result<(), EditorError> {
    let selected = ranges(view);
    let result = match command {
        EditCommand::Duplicate => Some(buture_editor::operations::duplicate_lines(
            buffer, &selected,
        )?),
        EditCommand::MoveUp | EditCommand::MoveDown => Some(buture_editor::operations::move_lines(
            buffer,
            &selected,
            matches!(command, EditCommand::MoveUp),
        )?),
        EditCommand::Comment => {
            let marker = match buffer
                .path
                .as_deref()
                .and_then(|p| p.extension())
                .and_then(|s| s.to_str())
            {
                Some("py" | "toml" | "yaml" | "yml" | "sh") => "#",
                _ => "//",
            };
            Some(buture_editor::operations::comment(
                buffer, &selected, marker,
            )?)
        }
        EditCommand::Indent | EditCommand::Outdent => Some(buture_editor::operations::indent(
            buffer,
            &selected,
            matches!(command, EditCommand::Outdent),
        )?),
        EditCommand::Occurrence => {
            if let Some(next) = buture_editor::operations::next_occurrence(buffer, &selected) {
                if view.selection.empty() && view.additional.is_empty() {
                    Some(std::iter::once(next).collect())
                } else {
                    let mut ranges = selected;
                    ranges.push(next);
                    Some(ranges)
                }
            } else {
                None
            }
        }
        EditCommand::CursorUp | EditCommand::CursorDown => {
            let mut next = view.selection.clone();
            move_caret(
                buffer,
                &mut next,
                if matches!(command, EditCommand::CursorUp) {
                    Key::ArrowUp
                } else {
                    Key::ArrowDown
                },
                false,
                false,
            );
            if next.head != view.selection.head {
                view.additional.push(view.selection.clone());
                view.selection = next;
            }
            None
        }
        EditCommand::ClearCursors => {
            view.additional.clear();
            None
        }
        EditCommand::Fold => {
            if let Some(parsed) = parsed {
                let row = buffer.char_to_line(view.selection.head)?;
                if let Some(range) = parsed.folds.iter().find(|r| r.start == row) {
                    if !view.folded.remove(&row) {
                        view.folded.insert(row);
                        view.selection.head = buffer.offset(
                            row,
                            buffer
                                .line(row)?
                                .trim_end_matches(['\r', '\n'])
                                .chars()
                                .count(),
                        )?;
                        view.selection.anchor = view.selection.head;
                    }
                    let _ = range;
                }
            }
            None
        }
        EditCommand::Undo | EditCommand::Redo => {
            if matches!(command, EditCommand::Undo) {
                buffer.undo();
            } else {
                buffer.redo();
            }
            view.selection.head = view.selection.head.min(buffer.len_chars());
            view.selection.anchor = view.selection.head;
            view.additional.clear();
            None
        }
        EditCommand::SelectAll => Some(std::iter::once(0..buffer.len_chars()).collect()),
    };
    if let Some(result) = result {
        assign(view, result);
    }
    Ok(())
}
fn input(ui: &mut Ui, buffer: &mut Buffer, view: &mut View) -> Result<bool, EditorError> {
    let mut save = false;
    let initial = (buffer.revision, view.selection.head);
    let events = ui.input(|i| i.events.clone());
    for event in events {
        match event {
            Event::Text(text) | Event::Ime(egui::ImeEvent::Commit(text)) => {
                let result = buture_editor::operations::insert(buffer, &ranges(view), &text, true)?;
                assign(view, result);
            }
            Event::Paste(text) => {
                let text = text
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
                    .replace('\n', buffer.line_ending.text());
                let result =
                    buture_editor::operations::insert(buffer, &ranges(view), &text, false)?;
                assign(view, result);
            }
            Event::Copy | Event::Cut => {
                let selected = ranges(view);
                let text = selected
                    .iter()
                    .map(|r| buffer.slice(r.clone()))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(buffer.line_ending.text());
                ui.ctx().copy_text(text);
                if matches!(event, Event::Cut) {
                    let result = buture_editor::operations::insert(buffer, &selected, "", false)?;
                    assign(view, result);
                }
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if modifiers.command
                    && modifiers.alt
                    && matches!(key, Key::ArrowUp | Key::ArrowDown)
                {
                    execute(
                        buffer,
                        view,
                        if key == Key::ArrowUp {
                            EditCommand::CursorUp
                        } else {
                            EditCommand::CursorDown
                        },
                        None,
                    )?;
                } else if modifiers.alt && matches!(key, Key::ArrowUp | Key::ArrowDown) {
                    execute(
                        buffer,
                        view,
                        if modifiers.shift {
                            EditCommand::Duplicate
                        } else if key == Key::ArrowUp {
                            EditCommand::MoveUp
                        } else {
                            EditCommand::MoveDown
                        },
                        None,
                    )?;
                } else if modifiers.command {
                    match key {
                        Key::S => save = true,
                        Key::A => execute(buffer, view, EditCommand::SelectAll, None)?,
                        Key::Z => execute(
                            buffer,
                            view,
                            if modifiers.shift {
                                EditCommand::Redo
                            } else {
                                EditCommand::Undo
                            },
                            None,
                        )?,
                        Key::Y => execute(buffer, view, EditCommand::Redo, None)?,
                        Key::D => execute(buffer, view, EditCommand::Occurrence, None)?,
                        Key::Slash => execute(buffer, view, EditCommand::Comment, None)?,
                        Key::OpenBracket => execute(buffer, view, EditCommand::Outdent, None)?,
                        Key::CloseBracket => execute(buffer, view, EditCommand::Indent, None)?,
                        _ => {
                            move_caret(buffer, &mut view.selection, key, modifiers.shift, true);
                            for selection in &mut view.additional {
                                move_caret(buffer, selection, key, modifiers.shift, true);
                            }
                        }
                    }
                } else {
                    match key {
                        Key::Enter => {
                            let result = buture_editor::operations::newline(buffer, &ranges(view))?;
                            assign(view, result);
                        }
                        Key::Tab => execute(
                            buffer,
                            view,
                            if modifiers.shift {
                                EditCommand::Outdent
                            } else {
                                EditCommand::Indent
                            },
                            None,
                        )?,
                        Key::Escape => view.additional.clear(),
                        Key::Backspace | Key::Delete => {
                            for selection in std::iter::once(&mut view.selection)
                                .chain(view.additional.iter_mut())
                            {
                                if selection.empty() {
                                    let head = selection.head;
                                    move_caret(
                                        buffer,
                                        selection,
                                        if key == Key::Backspace {
                                            Key::ArrowLeft
                                        } else {
                                            Key::ArrowRight
                                        },
                                        true,
                                        false,
                                    );
                                    selection.anchor = head;
                                }
                            }
                            let result = buture_editor::operations::insert(
                                buffer,
                                &ranges(view),
                                "",
                                false,
                            )?;
                            assign(view, result);
                        }
                        _ => {
                            move_caret(buffer, &mut view.selection, key, modifiers.shift, false);
                            for selection in &mut view.additional {
                                move_caret(buffer, selection, key, modifiers.shift, false);
                            }
                        }
                    }
                }
                view.follow_caret = true;
            }
            _ => {}
        }
    }
    if initial != (buffer.revision, view.selection.head) {
        view.follow_caret = true;
    }
    Ok(save)
}
fn hidden_rows(
    parsed: Option<&buture_editor::parsing::Parsed>,
    folded: &BTreeSet<usize>,
) -> Vec<Range<usize>> {
    let Some(parsed) = parsed else {
        return Vec::new();
    };
    let mut output: Vec<Range<usize>> = Vec::new();
    for start in folded {
        let first = parsed.folds.partition_point(|r| r.start < *start);
        if let Some(end) = parsed.folds[first..]
            .iter()
            .take_while(|r| r.start == *start)
            .map(|r| r.end)
            .max()
            && output.last().is_none_or(|r| r.end <= *start)
        {
            output.push(start + 1..end);
        }
    }
    output
}
fn source_row(visible: usize, hidden: &[Range<usize>]) -> usize {
    let mut row = visible;
    for range in hidden {
        if row >= range.start {
            row += range.len();
        } else {
            break;
        }
    }
    row
}
fn visible_row(source: usize, hidden: &[Range<usize>]) -> usize {
    source
        - hidden
            .iter()
            .filter(|r| r.end <= source)
            .map(Range::len)
            .sum::<usize>()
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
    render_parsed(ui, theme, buffer, view, None)
}
fn render_parsed(
    ui: &mut Ui,
    theme: &Theme,
    buffer: &mut Buffer,
    view: &mut View,
    parsed: Option<&buture_editor::parsing::Parsed>,
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
    let parsed =
        parsed.filter(|p| p.revision == buffer.revision && p.content_id == buffer.content_id());
    view.focus = Some(id);
    if view.fold_revision != buffer.revision {
        view.folded.clear();
        view.fold_revision = buffer.revision;
    }
    let mut hidden = hidden_rows(parsed, &view.folded);
    let caret_row = buffer.char_to_line(view.selection.head)?;
    if hidden.iter().any(|r| r.contains(&caret_row)) {
        view.folded.retain(|start| {
            !hidden
                .iter()
                .any(|r| r.start == start + 1 && r.contains(&caret_row))
        });
        hidden = hidden_rows(parsed, &view.folded);
    }
    let bracket_key = (buffer.revision, view.selection.head, parsed.is_some());
    if view
        .bracket_cache
        .as_ref()
        .is_none_or(|(key, _)| *key != bracket_key)
    {
        view.bracket_cache = Some((
            bracket_key,
            buture_editor::operations::matching_bracket(buffer, view.selection.head, parsed),
        ));
    }

    let char_width = ui.fonts_mut(|f| f.glyph_width(&font, 'M'));
    let mut scroll = egui::ScrollArea::both()
        .id_salt("code-scroll")
        .auto_shrink([false, false]);
    if view.follow_caret {
        let y = visible_row(caret_row, &hidden) as f32 * row_height;
        let height = ui.available_height() - 32.0;
        if y < view.scroll.y {
            scroll = scroll.vertical_scroll_offset(y);
        } else if y + row_height > view.scroll.y + height {
            scroll = scroll.vertical_scroll_offset((y + row_height - height).max(0.0));
        }
        let column = buffer.position(view.selection.head)?.1;
        let x = column as f32 * char_width + gutter;
        if x < view.scroll.x + gutter {
            scroll = scroll.horizontal_scroll_offset((x - gutter).max(0.0));
        } else if x > view.scroll.x + ui.available_width() - 24.0 {
            scroll = scroll.horizontal_scroll_offset((x - ui.available_width() + 48.0).max(0.0));
        }
        view.follow_caret = false;
    }
    if !ui.input(|i| i.pointer.primary_down()) {
        view.dragging = false;
    }
    let visible_count = buffer.line_count() - hidden.iter().map(Range::len).sum::<usize>();
    let output = scroll.show_rows(ui, row_height, visible_count, |ui, range| {
        ui.set_min_width(
            (buffer.width_hint.min(100_000) as f32 * char_width + gutter + 20.0)
                .max(ui.available_width()),
        );
        for visible in range {
            let row = source_row(visible, &hidden);
            let source = buffer.line(row).unwrap_or_default();
            let source = source.trim_end_matches(['\r', '\n']);
            let (display, map) = expand_tabs(source);
            let galley = ui.painter().layout_job(crate::syntax::layout_parsed(
                source,
                &display,
                &map,
                buffer
                    .char_to_byte(buffer.line_start(row).unwrap_or(0))
                    .unwrap_or(0),
                parsed,
                font.clone(),
                theme,
            ));
            let (rect, response) = ui.allocate_exact_size(
                vec2(ui.available_width(), row_height),
                Sense::click_and_drag(),
            );
            let text_pos = pos2(rect.left() + gutter, rect.top() + 2.0);
            let start = buffer.line_start(row).unwrap_or(0);
            if response.clicked() && ui.input(|i| i.modifiers.alt) {
                view.additional.push(view.selection.clone());
            } else if response.clicked() && !ui.input(|i| i.modifiers.shift) {
                view.additional.clear();
            }
            if response.drag_started() {
                view.dragging = true;
                if !ui.input(|i| i.modifiers.shift)
                    && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                {
                    let column = galley
                        .cursor_from_pos(origin - text_pos)
                        .index
                        .min(map.len() - 1);
                    view.selection.anchor = start + map[column];
                }
            }
            if (response.clicked()
                || response.dragged()
                || (view.dragging && ui.rect_contains_pointer(rect)))
                && let Some(pointer) = ui.input(|i| i.pointer.interact_pos())
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
            let end = start + source.chars().count();
            for selection in std::iter::once(&view.selection).chain(view.additional.iter()) {
                let selected = selection.range();
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
                    let col = selection.head - start;
                    let display_col = map.iter().rposition(|p| *p == col).unwrap_or(map.len() - 1);
                    let x = galley.pos_from_cursor(CCursor::new(display_col)).left() + text_pos.x;
                    ui.painter().line_segment(
                        [pos2(x, rect.top() + 2.0), pos2(x, rect.bottom() - 2.0)],
                        Stroke::new(1.5_f32, Color32::from(theme.accent.emerald)),
                    );
                }
            }
            if let Some((_, Some((a, b)))) = view.bracket_cache {
                for position in [a, b] {
                    if position >= start && position < end {
                        let index = map.iter().position(|p| *p == position - start).unwrap_or(0);
                        let rect = galley
                            .pos_from_cursor(CCursor::new(index))
                            .translate(text_pos.to_vec2())
                            .expand2(vec2(char_width * 0.5, 0.0));
                        ui.painter().rect_stroke(
                            rect,
                            2,
                            Stroke::new(1.0_f32, Color32::from(theme.accent.emerald)),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
            if let Some(parsed) = parsed {
                let first = parsed.folds.partition_point(|r| r.start < row);
                if parsed.folds.get(first).is_some_and(|r| r.start == row) {
                    let fold_rect = egui::Rect::from_min_size(
                        pos2(rect.left() + 4.0, rect.top()),
                        vec2(16.0, row_height),
                    );
                    let response = ui.interact(fold_rect, id.with(("fold", row)), Sense::click());
                    ui.painter().text(
                        fold_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        if view.folded.contains(&row) { "+" } else { "-" },
                        font.clone(),
                        theme.text.muted.into(),
                    );
                    if response.clicked() {
                        if !view.folded.remove(&row) {
                            view.folded.insert(row);
                            view.selection.head = start + source.chars().count();
                            view.selection.anchor = view.selection.head;
                            view.additional.clear();
                        }
                        ui.ctx().request_repaint();
                    }
                }
            }
            ui.painter()
                .galley(text_pos, galley, theme.text.primary.into());
        }
    });
    view.scroll = output.state.offset;
    let (row, col) = buffer.position(view.selection.head)?;
    ui.label(
        RichText::new(format!(
            "Ln {}, Col {} · {} cursor(s) · UTF-8 · {:?}",
            row + 1,
            col + 1,
            view.additional.len() + 1,
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
                parsed: None,
                parse_requested: None,
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
    #[test]
    fn split_views_share_buffers_and_move_tabs() {
        let mut space = EditorSpace::default();
        space.documents.push(Document {
            buffer: Buffer::from_text("shared"),
            view: View::default(),
            pinned: false,
            preview: false,
            parsed: None,
            parse_requested: None,
        });
        space.groups[0].tabs = vec![0];
        space.groups[0].active = Some(0);
        space.split(buture_editor::session::Orientation::Horizontal);
        assert_eq!(space.groups[1].tabs, vec![0]);
        space.documents[0]
            .buffer
            .insert(0, "edited ")
            .expect("edit");
        assert!(space.documents[space.groups[0].tabs[0]].buffer.dirty());
        space.action(TabAction::Move(0));
        assert!(space.groups[1].tabs.is_empty());
        assert_eq!(space.groups[0].tabs, vec![0]);
        assert_eq!(
            space.documents[0].buffer.rope().to_string(),
            "edited shared"
        );
    }
    #[test]
    fn occurrence_selection_replaces_all_cursors_and_undoes_once() {
        let mut buffer = Buffer::from_text("item item");
        let mut view = View {
            selection: Selection { anchor: 0, head: 4 },
            ..Default::default()
        };
        execute(&mut buffer, &mut view, EditCommand::Occurrence, None).expect("occurrence");
        assert_eq!(view.additional.len(), 1);
        let result = buture_editor::operations::insert(&mut buffer, &ranges(&view), "🦀", false)
            .expect("edit");
        assign(&mut view, result);
        assert_eq!(buffer.rope().to_string(), "🦀 🦀");
        execute(&mut buffer, &mut view, EditCommand::Undo, None).expect("undo");
        assert_eq!(buffer.rope().to_string(), "item item");
        assert!(!buffer.dirty());
    }
}
