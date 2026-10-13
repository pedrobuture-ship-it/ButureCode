//! All glass composition, blur policy and solid fallbacks live here.
use crate::{Theme, backdrop::Backdrop, theme::Color};
use buture_settings::VisualEffects;
use egui::{Color32, Frame, InnerResponse, Painter, Rect, Shape, Stroke, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassVariant {
    Subtle,
    Panel,
    Elevated,
    Floating,
    Modal,
}

#[derive(Debug, Clone, Copy)]
pub struct GlassSurface {
    pub variant: GlassVariant,
    pub blur: Option<u8>,
    pub opacity: Option<f32>,
    pub border: Option<Stroke>,
    pub radius: Option<u8>,
    pub shadow: Option<egui::epaint::Shadow>,
}

#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyle {
    pub fill: Color32,
    pub blur: u8,
    pub border: Stroke,
    pub radius: u8,
    pub shadow: egui::epaint::Shadow,
}

impl GlassSurface {
    pub const fn new(variant: GlassVariant) -> Self {
        Self {
            variant,
            blur: None,
            opacity: None,
            border: None,
            radius: None,
            shadow: None,
        }
    }
    pub fn blur(mut self, value: u8) -> Self {
        self.blur = Some(value);
        self
    }
    pub fn opacity(mut self, value: f32) -> Self {
        self.opacity = Some(value);
        self
    }
    pub fn border(mut self, value: Stroke) -> Self {
        self.border = Some(value);
        self
    }
    pub fn radius(mut self, value: u8) -> Self {
        self.radius = Some(value);
        self
    }
    pub fn shadow(mut self, value: egui::epaint::Shadow) -> Self {
        self.shadow = Some(value);
        self
    }

    pub fn resolve(self, theme: &Theme, effects: VisualEffects) -> SurfaceStyle {
        let (base, opacity, radius, elevated) = match self.variant {
            GlassVariant::Subtle => (
                theme.surface.glass,
                theme.opacity.subtle,
                theme.radius.small,
                false,
            ),
            GlassVariant::Panel => (
                theme.surface.panel,
                theme.opacity.panel,
                theme.radius.panel,
                false,
            ),
            GlassVariant::Elevated => (
                theme.surface.elevated,
                theme.opacity.elevated,
                theme.radius.panel,
                true,
            ),
            GlassVariant::Floating => (
                theme.surface.floating,
                theme.opacity.floating,
                theme.radius.floating,
                true,
            ),
            GlassVariant::Modal => (
                theme.surface.glass_strong,
                theme.opacity.modal,
                theme.radius.floating,
                true,
            ),
        };
        let (opacity, blur, shadow) = match effects {
            VisualEffects::Full => (
                self.opacity.unwrap_or(opacity),
                self.blur.unwrap_or(theme.blur.full),
                self.shadow.unwrap_or(if elevated {
                    theme.shadow.into()
                } else {
                    Default::default()
                }),
            ),
            VisualEffects::Reduced => (
                self.opacity.unwrap_or(opacity).max(theme.opacity.reduced),
                self.blur
                    .unwrap_or(theme.blur.reduced)
                    .min(theme.blur.reduced),
                Default::default(),
            ),
            VisualEffects::Off => (1.0, 0, Default::default()),
        };
        SurfaceStyle {
            fill: base.with_opacity(opacity).into(),
            blur,
            border: self.border.unwrap_or(Stroke::new(
                theme.border.width,
                Color32::from(theme.border.subtle),
            )),
            radius: self.radius.unwrap_or(radius),
            shadow,
        }
    }

    pub fn shapes(
        self,
        painter: &Painter,
        rect: Rect,
        viewport: Rect,
        theme: &Theme,
        effects: VisualEffects,
        backdrop: &mut Backdrop,
    ) -> Shape {
        let style = self.resolve(theme, effects);
        let mut shapes = Vec::with_capacity(4);
        if style.shadow != Default::default() {
            shapes.push(style.shadow.as_shape(rect, style.radius).into());
        }
        if effects != VisualEffects::Off {
            let texture = backdrop.texture(painter.ctx(), style.blur);
            shapes.push(
                egui::epaint::RectShape::filled(rect, style.radius, Color32::WHITE)
                    .with_texture(texture, Backdrop::uv(rect, viewport))
                    .into(),
            );
        }
        shapes.push(
            egui::epaint::RectShape::new(
                rect,
                style.radius,
                style.fill,
                style.border,
                egui::StrokeKind::Inside,
            )
            .into(),
        );
        Shape::Vec(shapes)
    }

    pub fn paint(
        self,
        painter: &Painter,
        rect: Rect,
        viewport: Rect,
        theme: &Theme,
        effects: VisualEffects,
        backdrop: &mut Backdrop,
    ) {
        painter.add(self.shapes(painter, rect, viewport, theme, effects, backdrop));
    }

    pub fn show<R>(
        self,
        ui: &mut Ui,
        theme: &Theme,
        effects: VisualEffects,
        backdrop: &mut Backdrop,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let index = ui.painter().add(Shape::Noop);
        let response = Frame::new()
            .inner_margin(theme.spacing.md)
            .show(ui, contents);
        let shape = self.shapes(
            ui.painter(),
            response.response.rect,
            ui.ctx().content_rect(),
            theme,
            effects,
            backdrop,
        );
        ui.painter().set(index, shape);
        response
    }
}

impl From<Color> for Color32 {
    fn from(color: Color) -> Self {
        Self::from_rgba_unmultiplied(color.0, color.1, color.2, color.3)
    }
}
impl From<crate::theme::Shadow> for egui::epaint::Shadow {
    fn from(shadow: crate::theme::Shadow) -> Self {
        Self {
            offset: shadow.offset,
            blur: shadow.blur,
            spread: shadow.spread,
            color: shadow.color.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn off_enforces_solid_surfaces_even_with_custom_effects() {
        let theme = Theme::dark();
        for variant in [
            GlassVariant::Subtle,
            GlassVariant::Panel,
            GlassVariant::Elevated,
            GlassVariant::Floating,
            GlassVariant::Modal,
        ] {
            let style = GlassSurface::new(variant)
                .opacity(0.1)
                .blur(50)
                .shadow(theme.shadow.into())
                .resolve(&theme, VisualEffects::Off);
            assert_eq!(style.fill.a(), 255);
            assert_eq!(style.blur, 0);
            assert_eq!(style.shadow, Default::default());
        }
    }
    #[test]
    fn reduced_caps_custom_blur_and_preserves_opacity() {
        let theme = Theme::dark();
        let style = GlassSurface::new(GlassVariant::Floating)
            .opacity(0.1)
            .blur(60)
            .resolve(&theme, VisualEffects::Reduced);
        assert_eq!(style.blur, theme.blur.reduced);
        assert!(style.fill.a() >= (255.0 * theme.opacity.reduced).round() as u8);
        assert_eq!(style.shadow, Default::default());
    }
}
