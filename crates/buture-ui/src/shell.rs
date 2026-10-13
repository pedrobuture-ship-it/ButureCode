//! Main IDE layout only. Editor, explorer and tool integrations belong to later prompts.
use crate::{Backdrop, GlassSurface, GlassVariant, Theme};
use buture_settings::VisualEffects;
use egui::{
    Align, Align2, Button, Color32, Context, FontId, Layout, Rect, RichText, Sense, Stroke, Ui,
    UiBuilder, Vec2, pos2, vec2,
};

#[derive(Debug, Clone, Copy)]
pub struct ShellLayout {
    pub outer: Rect,
    pub top: Rect,
    pub activity: Rect,
    pub sidebar: Rect,
    pub editor: Rect,
    pub inspector: Option<Rect>,
    pub bottom: Option<Rect>,
    pub status: Rect,
}

impl ShellLayout {
    pub fn calculate(viewport: Rect, theme: &Theme, inspector: bool, bottom: bool) -> Self {
        let outer = viewport.shrink(theme.spacing.sm);
        let top = Rect::from_min_size(outer.min, vec2(outer.width(), 56.0));
        let status = Rect::from_min_max(pos2(outer.left(), outer.bottom() - 28.0), outer.max);
        let body_top = top.bottom() + theme.spacing.xs;
        let body_bottom = status.top() - theme.spacing.xs;
        let activity = Rect::from_min_max(
            pos2(outer.left(), body_top),
            pos2(outer.left() + 64.0, body_bottom),
        );
        let sidebar_width = (outer.width() * 0.17).clamp(160.0, 224.0);
        let sidebar = Rect::from_min_max(
            pos2(activity.right() + theme.spacing.xs, body_top),
            pos2(
                activity.right() + theme.spacing.xs + sidebar_width,
                body_bottom,
            ),
        );
        let inspector = (inspector && viewport.width() >= 1150.0).then(|| {
            Rect::from_min_max(
                pos2(outer.right() - 254.0, body_top),
                pos2(outer.right(), body_bottom),
            )
        });
        let content_left = sidebar.right() + theme.spacing.xs;
        let content_right = inspector.map_or(outer.right(), |rect| rect.left() - theme.spacing.xs);
        let bottom = bottom.then(|| {
            let height = ((body_bottom - body_top) * 0.24).clamp(116.0, 184.0);
            Rect::from_min_max(
                pos2(content_left, body_bottom - height),
                pos2(content_right, body_bottom),
            )
        });
        let editor = Rect::from_min_max(
            pos2(content_left, body_top),
            pos2(
                content_right,
                bottom.map_or(body_bottom, |rect| rect.top() - theme.spacing.xs),
            ),
        );
        Self {
            outer,
            top,
            activity,
            sidebar,
            editor,
            inspector,
            bottom,
            status,
        }
    }
}

pub struct IdeShell {
    pub theme: Theme,
    pub effects: VisualEffects,
    pub show_inspector: bool,
    pub show_bottom_panel: bool,
    backdrop: Backdrop,
}

impl IdeShell {
    pub fn new(ctx: &Context) -> Self {
        let theme = Theme::dark();
        configure(ctx, &theme);
        Self {
            backdrop: Backdrop::new(ctx, &theme),
            theme,
            effects: VisualEffects::Full,
            show_inspector: true,
            show_bottom_panel: true,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        let theme = self.theme;
        let layout = ShellLayout::calculate(
            ctx.content_rect(),
            &theme,
            self.show_inspector,
            self.show_bottom_panel,
        );
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                if self.effects == VisualEffects::Off {
                    ui.painter().rect_filled(
                        ctx.content_rect(),
                        0,
                        Color32::from(theme.background.base),
                    );
                } else {
                    self.backdrop.paint(ui.painter(), ctx.content_rect());
                }
                GlassSurface::new(GlassVariant::Panel)
                    .radius(theme.radius.window)
                    .border(Stroke::new(
                        theme.border.width,
                        Color32::from(theme.border.normal),
                    ))
                    .paint(
                        ui.painter(),
                        layout.outer,
                        ctx.content_rect(),
                        &theme,
                        self.effects,
                        &mut self.backdrop,
                    );
                for rect in [layout.top, layout.activity, layout.sidebar, layout.status]
                    .into_iter()
                    .chain(layout.inspector)
                    .chain(layout.bottom)
                {
                    GlassSurface::new(GlassVariant::Subtle)
                        .radius(theme.radius.panel)
                        .paint(
                            ui.painter(),
                            rect,
                            ctx.content_rect(),
                            &theme,
                            self.effects,
                            &mut self.backdrop,
                        );
                }
                // Explicitly solid. No glass or backdrop texture behind editor content.
                ui.painter().rect(
                    layout.editor,
                    theme.radius.panel,
                    Color32::from(theme.background.editor),
                    Stroke::new(theme.border.width, Color32::from(theme.border.subtle)),
                    egui::StrokeKind::Inside,
                );
                self.top_bar(&mut region(
                    ui,
                    "top",
                    layout.top.shrink2(vec2(theme.spacing.lg, theme.spacing.sm)),
                ));
                activity_bar(
                    &mut region(
                        ui,
                        "activity",
                        layout
                            .activity
                            .shrink2(vec2(theme.spacing.sm, theme.spacing.md)),
                    ),
                    &theme,
                );
                sidebar(
                    &mut region(ui, "sidebar", layout.sidebar.shrink(theme.spacing.lg)),
                    &theme,
                );
                editor(&mut region(ui, "editor", layout.editor), &theme);
                if let Some(rect) = layout.inspector {
                    self.inspector(&mut region(ui, "inspector", rect.shrink(theme.spacing.xl)));
                }
                if let Some(rect) = layout.bottom {
                    output_panel(
                        &mut region(ui, "output", rect.shrink(theme.spacing.lg)),
                        &theme,
                    );
                }
                status_bar(
                    &mut region(
                        ui,
                        "status",
                        layout
                            .status
                            .shrink2(vec2(theme.spacing.lg, theme.spacing.xs)),
                    ),
                    &theme,
                );
            });
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        let theme = self.theme;
        ui.horizontal_centered(|ui| {
            mark(ui, 26.0, &theme);
            ui.add_space(theme.spacing.sm);
            ui.label(
                RichText::new("Buture Code")
                    .size(theme.typography.title)
                    .strong()
                    .color(theme.text.primary),
            );
            ui.add_space(theme.spacing.xl);
            ui.label(
                RichText::new("WORKSPACE")
                    .size(theme.typography.caption)
                    .color(theme.text.muted),
            );
            ui.label(RichText::new("/  No folder open").color(theme.text.secondary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(Button::new("×").frame(false))
                    .on_hover_text("Close window")
                    .clicked()
                {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ui
                    .add(Button::new("□").frame(false))
                    .on_hover_text("Maximize / restore")
                    .clicked()
                {
                    let maximized = ui
                        .ctx()
                        .input(|input| input.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                if ui
                    .add(Button::new("−").frame(false))
                    .on_hover_text("Minimize")
                    .clicked()
                {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                ui.separator();
                ui.toggle_value(&mut self.show_inspector, "Inspector")
                    .on_hover_text("Toggle right inspector");
                ui.toggle_value(&mut self.show_bottom_panel, "Panel")
                    .on_hover_text("Toggle output panel");
                let drag =
                    ui.allocate_response(vec2(ui.available_width().max(0.0), 32.0), Sense::drag());
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let maximized = ui
                        .ctx()
                        .input(|input| input.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
            });
        });
    }

    fn inspector(&mut self, ui: &mut Ui) {
        let theme = self.theme;
        ui.add_space(theme.spacing.lg);
        mark(ui, 42.0, &theme);
        ui.add_space(theme.spacing.xl);
        ui.label(
            RichText::new("Buture Code")
                .size(theme.typography.title)
                .strong()
                .color(theme.text.primary),
        );
        ui.add_space(theme.spacing.sm);
        ui.label(
            RichText::new("An engineering workspace.\nBuilt around your environment.")
                .color(theme.text.secondary),
        );
        ui.add_space(theme.spacing.xxl);
        caption(ui, "APPLICATION", &theme);
        ui.add_space(theme.spacing.md);
        detail(ui, "Version", "0.1.0", &theme);
        detail(ui, "Author", "Pedro Buture", &theme);
        detail(ui, "Runtime", "Native Rust", &theme);
        ui.add_space(theme.spacing.xl);
        ui.separator();
        ui.add_space(theme.spacing.xl);
        caption(ui, "VISUAL EFFECTS", &theme);
        ui.add_space(theme.spacing.md);
        ui.horizontal(|ui| {
            for effects in VisualEffects::ALL {
                ui.selectable_value(&mut self.effects, effects, effects.label());
            }
        });
        ui.add_space(theme.spacing.sm);
        ui.label(
            RichText::new(match self.effects {
                VisualEffects::Full => "Smoked glass. Subtle depth.",
                VisualEffects::Reduced => "Less blur. The same focus.",
                VisualEffects::Off => "Solid surfaces. Clear contrast.",
            })
            .size(theme.typography.caption)
            .color(theme.text.muted),
        );
    }
}

fn region(parent: &mut Ui, id: &str, rect: Rect) -> Ui {
    let mut child = parent.new_child(
        UiBuilder::new()
            .id_salt(id)
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(rect);
    child
}

fn configure(ctx: &Context, theme: &Theme) {
    let mut style = egui::Style {
        visuals: egui::Visuals::dark(),
        ..Default::default()
    };
    style.visuals.override_text_color = Some(theme.text.primary.into());
    style.visuals.panel_fill = theme.background.base.into();
    style.visuals.window_fill = theme.surface.floating.into();
    style.visuals.extreme_bg_color = theme.background.deep.into();
    style.visuals.faint_bg_color = theme.surface.panel.into();
    style.visuals.selection.bg_fill = theme.accent.emerald_muted.into();
    style.visuals.selection.stroke =
        Stroke::new(theme.border.width, Color32::from(theme.accent.emerald));
    style.visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(theme.border.width, Color32::from(theme.border.subtle));
    style.visuals.widgets.inactive.weak_bg_fill = theme.surface.panel.into();
    style.visuals.widgets.inactive.bg_fill = theme.surface.panel.into();
    style.visuals.widgets.inactive.bg_stroke =
        Stroke::new(theme.border.width, Color32::from(theme.border.subtle));
    style.visuals.widgets.inactive.fg_stroke.color = theme.text.secondary.into();
    style.visuals.widgets.hovered.weak_bg_fill = theme.surface.elevated.into();
    style.visuals.widgets.hovered.bg_fill = theme.surface.elevated.into();
    style.visuals.widgets.hovered.bg_stroke.color = theme.border.normal.into();
    style.visuals.widgets.hovered.fg_stroke.color = theme.text.primary.into();
    style.visuals.widgets.active.weak_bg_fill = theme.accent.emerald_muted.into();
    style.visuals.widgets.active.bg_fill = theme.accent.emerald_muted.into();
    style.visuals.widgets.active.bg_stroke.color = theme.border.focus.into();
    style.visuals.widgets.active.fg_stroke.color = theme.accent.emerald.into();
    style.visuals.warn_fg_color = theme.semantic.warning.into();
    style.visuals.error_fg_color = theme.semantic.error.into();
    style.visuals.hyperlink_color = theme.accent.emerald.into();
    style.spacing.item_spacing = vec2(theme.spacing.sm, theme.spacing.sm);
    style.spacing.button_padding = vec2(theme.spacing.sm, theme.spacing.xs);
    style.animation_time = theme.motion.fast.as_secs_f32();
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::proportional(theme.typography.body),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::proportional(theme.typography.body),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::monospace(theme.typography.code),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::proportional(theme.typography.caption),
    );
    ctx.set_style(style);
}

fn caption(ui: &mut Ui, text: &str, theme: &Theme) {
    ui.label(
        RichText::new(text)
            .size(theme.typography.caption)
            .color(theme.text.muted),
    );
}
fn detail(ui: &mut Ui, key: &str, value: &str, theme: &Theme) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).color(theme.text.muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(value);
        });
    });
}
fn mark(ui: &mut Ui, size: f32, theme: &Theme) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    ui.painter().rect(
        rect,
        theme.radius.small,
        Color32::from(theme.accent.emerald_muted),
        Stroke::new(theme.border.width, Color32::from(theme.accent.emerald)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "B",
        FontId::proportional(size * 0.65),
        theme.accent.emerald.into(),
    );
}

fn activity_bar(ui: &mut Ui, theme: &Theme) {
    for (index, (icon, label)) in [
        (Icon::Folder, "Explorer"),
        (Icon::Search, "Search"),
        (Icon::Branch, "Source Control"),
        (Icon::Play, "Run & Debug"),
        (Icon::Database, "Database"),
        (Icon::Box, "Docker"),
        (Icon::Chip, "Hardware"),
    ]
    .into_iter()
    .enumerate()
    {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::hover());
        response.on_hover_text(label);
        let selected = index == 0;
        if selected {
            ui.painter().rect(
                rect,
                theme.radius.small,
                Color32::from(theme.accent.emerald_muted),
                Stroke::new(theme.border.width, Color32::from(theme.border.subtle)),
                egui::StrokeKind::Inside,
            );
        }
        icon.paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(20.0)),
            if selected {
                theme.accent.emerald.into()
            } else {
                theme.text.muted.into()
            },
        );
    }
    ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
        response.on_hover_text("Settings");
        Icon::Settings.paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(20.0)),
            theme.text.muted.into(),
        );
    });
}
fn sidebar(ui: &mut Ui, theme: &Theme) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Explorer").strong().color(theme.text.primary));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new("...").color(theme.text.muted));
        });
    });
    ui.add_space(theme.spacing.lg);
    ui.separator();
    ui.add_space(theme.spacing.xl);
    caption(ui, "WORKSPACE", theme);
    ui.add_space(theme.spacing.md);
    ui.label(RichText::new("No folder open").color(theme.text.secondary));
    ui.add_space(theme.spacing.sm);
    ui.label(
        RichText::new("Your project files belong here.")
            .size(theme.typography.caption)
            .color(theme.text.muted),
    );
}
fn editor(ui: &mut Ui, theme: &Theme) {
    let rect = ui.max_rect();
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), 40.0));
    ui.painter().line_segment(
        [header.left_bottom(), header.right_bottom()],
        Stroke::new(theme.border.width, Color32::from(theme.border.subtle)),
    );
    let mut tabs = region(
        ui,
        "tab-header",
        header.shrink2(vec2(theme.spacing.lg, theme.spacing.sm)),
    );
    tabs.horizontal(|ui| {
        ui.label(RichText::new("Welcome").color(theme.text.primary));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            caption(ui, "BUTURE GLASS DARK", theme);
        });
    });
    let content = Rect::from_min_max(
        pos2(
            rect.left() + theme.spacing.xxl,
            header.bottom() + theme.spacing.xl,
        ),
        pos2(
            rect.right() - theme.spacing.xxl,
            rect.bottom() - theme.spacing.xl,
        ),
    );
    let mut center = region(ui, "welcome-content", content);
    egui::ScrollArea::vertical()
        .id_salt("welcome-scroll")
        .show(&mut center, |ui| {
            ui.add_space((content.height() * 0.15).min(64.0));
            ui.vertical_centered(|ui| {
                mark(ui, 56.0, theme);
                ui.add_space(theme.spacing.xl);
                ui.label(
                    RichText::new("Buture Code")
                        .size(theme.typography.display)
                        .strong()
                        .color(theme.text.primary),
                );
                ui.add_space(theme.spacing.sm);
                ui.label(
                    RichText::new("Engineering without friction.")
                        .size(theme.typography.title)
                        .color(theme.text.secondary),
                );
                ui.add_space(theme.spacing.xxl);
                ui.label(
                    RichText::new("A clear space to build what comes next.")
                        .color(theme.text.muted),
                );
                ui.add_space(theme.spacing.xxl);
            });
            let width = ui.available_width().min(420.0);
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.separator();
                    ui.add_space(theme.spacing.lg);
                    caption(ui, "ENGINEERING STARTS WITH A STRONG FOUNDATION", theme);
                    ui.add_space(theme.spacing.md);
                    for (number, line) in [
                        "// Buture Code · Pedro Buture",
                        "fn main() {",
                        "    // A workspace built for engineering.",
                        "}",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{:>2}", number + 1))
                                    .monospace()
                                    .color(theme.text.disabled),
                            );
                            ui.add_space(theme.spacing.lg);
                            ui.label(RichText::new(line).monospace().color(theme.text.secondary));
                        });
                    }
                });
            });
        });
}
fn output_panel(ui: &mut Ui, theme: &Theme) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Output").color(theme.accent.emerald));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            caption(ui, "APPLICATION", theme);
        });
    });
    ui.add_space(theme.spacing.sm);
    ui.separator();
    ui.add_space(theme.spacing.md);
    egui::ScrollArea::vertical()
        .id_salt("output-scroll")
        .show(ui, |ui| {
            ui.label(
                RichText::new("Buture Code 0.1.0")
                    .monospace()
                    .color(theme.text.secondary),
            );
            ui.label(
                RichText::new("Native window initialized.")
                    .monospace()
                    .color(theme.text.muted),
            );
            ui.label(
                RichText::new("No workspace open.")
                    .monospace()
                    .color(theme.text.muted),
            );
        });
}
fn status_bar(ui: &mut Ui, theme: &Theme) {
    ui.horizontal_centered(|ui| {
        let (dot, _) = ui.allocate_exact_size(vec2(8.0, 12.0), Sense::hover());
        ui.painter()
            .circle_filled(dot.center(), 2.5, Color32::from(theme.accent.emerald));
        ui.label(
            RichText::new("Shell ready")
                .size(theme.typography.caption)
                .color(theme.text.secondary),
        );
        ui.add_space(theme.spacing.lg);
        caption(ui, "No workspace", theme);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            caption(ui, "Buture Code  /  0.1.0", theme);
        });
    });
}

// Original vector primitives; no borrowed product icon set.
#[derive(Clone, Copy)]
enum Icon {
    Folder,
    Search,
    Branch,
    Play,
    Database,
    Box,
    Chip,
    Settings,
}
impl Icon {
    fn paint(self, painter: &egui::Painter, rect: Rect, color: Color32) {
        let point = |x: f32, y: f32| {
            pos2(
                rect.left() + rect.width() * x,
                rect.top() + rect.height() * y,
            )
        };
        let stroke = Stroke::new(1.4_f32, color);
        let line = |points: &[(f32, f32)]| {
            painter.add(egui::Shape::line(
                points.iter().map(|&(x, y)| point(x, y)).collect(),
                stroke,
            ));
        };
        match self {
            Self::Folder => line(&[
                (0.08, 0.85),
                (0.08, 0.20),
                (0.40, 0.20),
                (0.52, 0.35),
                (0.92, 0.35),
                (0.92, 0.85),
                (0.08, 0.85),
            ]),
            Self::Search => {
                painter.circle_stroke(point(0.42, 0.42), rect.width() * 0.32, stroke);
                line(&[(0.66, 0.66), (0.94, 0.94)]);
            }
            Self::Branch => {
                line(&[(0.25, 0.2), (0.25, 0.8)]);
                line(&[(0.25, 0.6), (0.70, 0.45), (0.70, 0.2)]);
                for (x, y) in [(0.25, 0.15), (0.25, 0.85), (0.70, 0.15)] {
                    painter.circle_stroke(point(x, y), rect.width() * 0.10, stroke);
                }
            }
            Self::Play => line(&[(0.25, 0.12), (0.88, 0.50), (0.25, 0.88), (0.25, 0.12)]),
            Self::Database => {
                for y in [0.2, 0.5, 0.8] {
                    painter.add(egui::Shape::ellipse_stroke(
                        point(0.5, y),
                        vec2(rect.width() * 0.37, rect.height() * 0.14),
                        stroke,
                    ));
                }
                line(&[(0.13, 0.2), (0.13, 0.8)]);
                line(&[(0.87, 0.2), (0.87, 0.8)]);
            }
            Self::Box => {
                line(&[
                    (0.12, 0.28),
                    (0.50, 0.08),
                    (0.88, 0.28),
                    (0.88, 0.72),
                    (0.50, 0.92),
                    (0.12, 0.72),
                    (0.12, 0.28),
                    (0.50, 0.50),
                    (0.88, 0.28),
                ]);
                line(&[(0.5, 0.5), (0.5, 0.92)]);
            }
            Self::Chip => {
                painter.rect_stroke(
                    rect.shrink(rect.width() * 0.22),
                    2,
                    stroke,
                    egui::StrokeKind::Inside,
                );
                for v in [0.32, 0.5, 0.68] {
                    for points in [
                        [(v, 0.05), (v, 0.22)],
                        [(v, 0.78), (v, 0.95)],
                        [(0.05, v), (0.22, v)],
                        [(0.78, v), (0.95, v)],
                    ] {
                        line(&points);
                    }
                }
            }
            Self::Settings => {
                painter.circle_stroke(rect.center(), rect.width() * 0.32, stroke);
                painter.circle_stroke(rect.center(), rect.width() * 0.12, stroke);
                for n in 0..8 {
                    let angle = n as f32 * std::f32::consts::TAU / 8.0;
                    let dir = vec2(angle.cos(), angle.sin());
                    painter.line_segment(
                        [
                            rect.center() + dir * rect.width() * 0.32,
                            rect.center() + dir * rect.width() * 0.46,
                        ],
                        stroke,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_keeps_space_with_optional_panels() {
        let theme = Theme::dark();
        for size in [
            vec2(960.0, 640.0),
            vec2(1280.0, 720.0),
            vec2(1440.0, 900.0),
            vec2(1920.0, 1080.0),
        ] {
            for inspector in [false, true] {
                for bottom in [false, true] {
                    let layout = ShellLayout::calculate(
                        Rect::from_min_size(egui::Pos2::ZERO, size),
                        &theme,
                        inspector,
                        bottom,
                    );
                    assert!(layout.editor.width() >= 400.0);
                    assert!(layout.editor.height() >= 300.0);
                    assert!(!layout.editor.intersects(layout.sidebar));
                    if let Some(rect) = layout.inspector {
                        assert!(!layout.editor.intersects(rect));
                    }
                    if let Some(rect) = layout.bottom {
                        assert!(!layout.editor.intersects(rect));
                    }
                }
            }
        }
    }
    #[test]
    fn shell_renders_all_effect_modes_without_platform_io() {
        let ctx = Context::default();
        let mut shell = IdeShell::new(&ctx);
        for effects in VisualEffects::ALL {
            shell.effects = effects;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1440.0, 900.0))),
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| shell.show(ctx));
            assert!(!output.shapes.is_empty());
            assert!(
                !ctx.tessellate(output.shapes, output.pixels_per_point)
                    .is_empty()
            );
        }
    }
}
