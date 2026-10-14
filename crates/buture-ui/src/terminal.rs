//! Solid terminal canvas; shell startup is an explicit user action.
use crate::Theme;
use buture_terminal::{Color, Event, Screen, Session};
use egui::{FontId, RichText, Ui};
use std::path::PathBuf;
struct Tab {
    name: String,
    session: Session,
    screen: Option<Box<Screen>>,
    cwd: PathBuf,
    state: String,
    size: (u16, u16),
    scroll: usize,
}
#[derive(Default)]
pub struct Terminals {
    tabs: Vec<Tab>,
    active: usize,
    split: bool,
    error: Option<String>,
}
impl Terminals {
    pub fn start(&mut self, root: Option<&std::path::Path>) {
        let cwd = root.map(PathBuf::from).or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
        });
        if let Some(cwd) = cwd {
            let name = format!("Shell {}", self.tabs.len() + 1);
            self.tabs.push(Tab {
                name,
                session: Session::start(cwd.clone()),
                screen: None,
                cwd,
                state: "Starting…".into(),
                size: (24, 80),
                scroll: 0,
            });
            self.active = self.tabs.len() - 1;
        } else {
            self.error = Some("No shell working directory available".into());
        }
    }
    pub fn running(&self) -> bool {
        self.tabs
            .iter()
            .any(|t| !t.state.starts_with("Exited") && !t.state.starts_with("Failed"))
    }
    pub fn poll(&mut self) {
        for tab in &mut self.tabs {
            while let Ok(event) = tab.session.events.try_recv() {
                match event {
                    Event::Started { shell, cwd } => {
                        tab.state = format!("{}", shell.display());
                        tab.cwd = cwd;
                    }
                    Event::Screen(screen) => tab.screen = Some(screen),
                    Event::Exited(code) => tab.state = format!("Exited {code}"),
                    Event::Failed(error) => tab.state = format!("Failed: {error}"),
                }
            }
        }
    }
    pub fn show(&mut self, ui: &mut Ui, theme: &Theme, root: Option<&std::path::Path>) {
        self.poll();
        ui.horizontal(|ui| {
            ui.heading("Terminal");
            if ui.button("New shell").clicked() {
                self.start(root);
            }
            ui.checkbox(&mut self.split, "Split");
            for (i, tab) in self.tabs.iter().enumerate() {
                ui.selectable_value(&mut self.active, i, &tab.name);
            }
        });
        if let Some(error) = &self.error {
            ui.label(error);
        }
        if self.tabs.is_empty() {
            ui.label("Start a shell explicitly. Terminal commands run with your user permissions.");
            return;
        }
        let active = self.active.min(self.tabs.len() - 1);
        let other = active.saturating_sub(1);
        if self.split && other != active {
            ui.columns(2, |uis| {
                Self::canvas(&mut self.tabs[other], &mut uis[0], theme, other);
                Self::canvas(&mut self.tabs[active], &mut uis[1], theme, active);
            });
        } else {
            Self::canvas(&mut self.tabs[active], ui, theme, active);
        }
    }
    fn canvas(tab: &mut Tab, ui: &mut Ui, theme: &Theme, _id: usize) {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut tab.name).desired_width(130.0));
            if ui.button("Kill").clicked() {
                tab.session.kill();
            }
            if ui.button("Copy screen").clicked()
                && let Some(screen) = &tab.screen
            {
                ui.ctx().copy_text(screen.contents());
            }
        });
        ui.label(format!("{} · {}", tab.cwd.display(), tab.state));
        egui::Frame::new()
            .fill(theme.background.editor.into())
            .show(ui, |ui| {
                let font = FontId::monospace(theme.typography.code);
                let height = ui.fonts_mut(|f| f.row_height(&font));
                let width = ui.fonts_mut(|f| f.glyph_width(&font, 'M'));
                let size = (
                    (ui.available_height() / height).floor().clamp(1.0, 200.0) as u16,
                    (ui.available_width() / width).floor().clamp(1.0, 400.0) as u16,
                );
                if tab.size != size {
                    tab.size = size;
                    tab.session.resize(size.0, size.1);
                }
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
                if response.clicked() {
                    response.request_focus();
                }
                if response.hovered() {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll != 0.0 {
                        tab.scroll = (tab.scroll as isize + (scroll / height) as isize)
                            .clamp(0, 2000) as usize;
                        tab.session.scroll(tab.scroll);
                    }
                }
                if let Some(screen) = &tab.screen {
                    for row in 0..size.0.min(screen.size().0) {
                        let mut job = egui::text::LayoutJob::default();
                        for col in 0..size.1.min(screen.size().1) {
                            if let Some(cell) = screen.cell(row, col) {
                                if cell.is_wide_continuation() {
                                    continue;
                                }
                                let content = cell.contents();
                                let text = if content.is_empty() { " " } else { content };
                                let (mut fg, mut bg) = (
                                    color(cell.fgcolor(), theme, false),
                                    color(cell.bgcolor(), theme, true),
                                );
                                if cell.inverse() {
                                    std::mem::swap(&mut fg, &mut bg);
                                }
                                job.append(
                                    text,
                                    0.0,
                                    egui::TextFormat {
                                        font_id: font.clone(),
                                        color: fg,
                                        background: bg,
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                        let galley = ui.fonts_mut(|f| f.layout_job(job));
                        ui.painter().galley(
                            rect.min + egui::vec2(0.0, row as f32 * height),
                            galley,
                            theme.text.primary.into(),
                        );
                    }
                    if response.has_focus() && !screen.hide_cursor() && tab.scroll == 0 {
                        let (row, col) = screen.cursor_position();
                        let cursor = egui::Rect::from_min_size(
                            rect.min + egui::vec2(col as f32 * width, row as f32 * height),
                            egui::vec2(width, height),
                        );
                        ui.painter().rect_stroke(
                            cursor,
                            0,
                            egui::Stroke::new(1.0_f32, egui::Color32::from(theme.accent.emerald)),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                if response.has_focus() {
                    for event in ui.input(|i| i.events.clone()) {
                        let bytes = match event {
                            egui::Event::Text(text) => Some(text.into_bytes()),
                            egui::Event::Copy => Some(vec![3]),
                            egui::Event::Cut => Some(vec![24]),
                            egui::Event::Paste(text) => Some(
                                if tab.screen.as_ref().is_some_and(|s| s.bracketed_paste()) {
                                    format!("\x1b[200~{text}\x1b[201~").into_bytes()
                                } else {
                                    text.into_bytes()
                                },
                            ),
                            egui::Event::Key {
                                key,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                if modifiers.ctrl && !modifiers.shift {
                                    let name = format!("{key:?}");
                                    if name.len() == 1 && name.as_bytes()[0].is_ascii_uppercase() {
                                        Some(vec![name.as_bytes()[0] - b'A' + 1])
                                    } else {
                                        key_bytes(key)
                                    }
                                } else {
                                    key_bytes(key)
                                }
                            }
                            _ => None,
                        };
                        if let Some(bytes) = bytes {
                            let _ = tab.session.input(bytes);
                            tab.scroll = 0;
                        }
                    }
                }
            });
        ui.label(
            RichText::new(
                "Click the screen to type · wheel for scrollback · copy screen uses the clipboard",
            )
            .color(theme.text.muted),
        );
    }
}
fn key_bytes(key: egui::Key) -> Option<Vec<u8>> {
    Some(match key {
        egui::Key::Enter => b"\r".to_vec(),
        egui::Key::Backspace => vec![127],
        egui::Key::Tab => vec![9],
        egui::Key::Escape => vec![27],
        egui::Key::ArrowUp => b"\x1b[A".to_vec(),
        egui::Key::ArrowDown => b"\x1b[B".to_vec(),
        egui::Key::ArrowRight => b"\x1b[C".to_vec(),
        egui::Key::ArrowLeft => b"\x1b[D".to_vec(),
        egui::Key::Home => b"\x1b[H".to_vec(),
        egui::Key::End => b"\x1b[F".to_vec(),
        egui::Key::Delete => b"\x1b[3~".to_vec(),
        egui::Key::PageUp => b"\x1b[5~".to_vec(),
        egui::Key::PageDown => b"\x1b[6~".to_vec(),
        _ => return None,
    })
}
fn color(value: Color, theme: &Theme, background: bool) -> egui::Color32 {
    match value {
        Color::Default => {
            if background {
                theme.background.editor.into()
            } else {
                theme.text.primary.into()
            }
        }
        Color::Rgb(r, g, b) => egui::Color32::from_rgb(r, g, b),
        Color::Idx(index) => {
            let palette = [
                theme.background.editor,
                theme.semantic.error,
                theme.semantic.success,
                theme.semantic.warning,
                theme.semantic.info,
                theme.syntax.keyword,
                theme.syntax.function,
                theme.text.primary,
                theme.text.muted,
                theme.semantic.error,
                theme.accent.emerald,
                theme.semantic.warning,
                theme.semantic.info,
                theme.syntax.keyword,
                theme.syntax.function,
                theme.text.primary,
            ];
            if index < 16 {
                palette[index as usize].into()
            } else if index >= 232 {
                egui::Color32::from_gray(8 + (index - 232) * 10)
            } else {
                let n = index - 16;
                let component = |c: u8| if c == 0 { 0 } else { 55 + c * 40 };
                egui::Color32::from_rgb(component(n / 36), component(n / 6 % 6), component(n % 6))
            }
        }
    }
}
