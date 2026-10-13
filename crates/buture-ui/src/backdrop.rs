//! One cached, app-owned backdrop. No OS wallpaper capture or per-widget blur.
use crate::Theme;
use egui::{Color32, ColorImage, Context, Rect, TextureHandle, TextureOptions};
use std::collections::BTreeMap;

pub struct Backdrop {
    source: ColorImage,
    textures: BTreeMap<u8, TextureHandle>,
}

impl Backdrop {
    pub fn new(ctx: &Context, theme: &Theme) -> Self {
        let [width, height] = theme.blur.backdrop_size;
        let mut source = ColorImage::filled([width, height], theme.background.base.into());
        for y in 0..height {
            for x in 0..width {
                let fx = x as f32 / width as f32;
                let fy = y as f32 / height as f32;
                let emerald =
                    (-((fx - 0.12).powi(2) * 14.0 + (fy - 0.75).powi(2) * 7.0)).exp() * 0.24;
                let graphite =
                    (-((fx - 0.75).powi(2) * 6.0 + (fy - 0.10).powi(2) * 12.0)).exp() * 0.7;
                let base = theme.background.base;
                let top = theme.surface.floating;
                let accent = theme.accent.emerald_muted;
                let blend = |a: u8, b: u8, c: u8| {
                    ((a as f32 * (1.0 - graphite) + b as f32 * graphite) * (1.0 - emerald)
                        + c as f32 * emerald)
                        .round() as u8
                };
                source.pixels[y * width + x] = Color32::from_rgb(
                    blend(base.0, top.0, accent.0),
                    blend(base.1, top.1, accent.1),
                    blend(base.2, top.2, accent.2),
                );
            }
        }
        let mut backdrop = Self {
            source,
            textures: BTreeMap::new(),
        };
        for radius in [0, theme.blur.full, theme.blur.reduced] {
            backdrop.texture(ctx, radius);
        }
        backdrop
    }

    pub fn texture(&mut self, ctx: &Context, radius: u8) -> egui::TextureId {
        self.textures
            .entry(radius)
            .or_insert_with(|| {
                ctx.load_texture(
                    format!("buture-backdrop-{radius}"),
                    gaussian_blur(&self.source, radius),
                    TextureOptions::LINEAR,
                )
            })
            .id()
    }

    pub fn paint(&mut self, painter: &egui::Painter, rect: Rect) {
        painter.image(
            self.texture(painter.ctx(), 0),
            rect,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    pub fn uv(rect: Rect, viewport: Rect) -> Rect {
        let map = |p: egui::Pos2| {
            egui::pos2(
                (p.x - viewport.min.x) / viewport.width().max(1.0),
                (p.y - viewport.min.y) / viewport.height().max(1.0),
            )
        };
        Rect::from_min_max(map(rect.min), map(rect.max))
    }
}

fn gaussian_blur(source: &ColorImage, radius: u8) -> ColorImage {
    if radius == 0 {
        return source.clone();
    }
    let radius = radius as isize;
    let sigma = (radius as f32 / 2.5).max(0.5);
    let mut kernel: Vec<f32> = (-radius..=radius)
        .map(|i| (-(i as f32).powi(2) / (2.0 * sigma * sigma)).exp())
        .collect();
    let sum: f32 = kernel.iter().sum();
    for weight in &mut kernel {
        *weight /= sum;
    }
    let [width, height] = source.size;
    let mut temp = vec![[0.0; 3]; width * height];
    for y in 0..height {
        for x in 0..width {
            for (i, weight) in kernel.iter().enumerate() {
                let px = (x as isize + i as isize - radius).clamp(0, width as isize - 1) as usize;
                let color = source.pixels[y * width + px];
                for (channel, value) in [color.r(), color.g(), color.b()].into_iter().enumerate() {
                    temp[y * width + x][channel] += value as f32 * weight;
                }
            }
        }
    }
    let mut output = source.clone();
    for y in 0..height {
        for x in 0..width {
            let mut channels = [0.0; 3];
            for (i, weight) in kernel.iter().enumerate() {
                let py = (y as isize + i as isize - radius).clamp(0, height as isize - 1) as usize;
                for (channel, value) in channels.iter_mut().enumerate() {
                    *value += temp[py * width + x][channel] * weight;
                }
            }
            output.pixels[y * width + x] = Color32::from_rgb(
                channels[0].round() as u8,
                channels[1].round() as u8,
                channels[2].round() as u8,
            );
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gaussian_softens_edges_and_preserves_solid_fields() {
        let solid = ColorImage::filled([7, 5], Color32::from_rgb(32, 64, 96));
        assert_eq!(gaussian_blur(&solid, 3).pixels, solid.pixels);
        let mut edge = ColorImage::filled([7, 1], Color32::BLACK);
        for pixel in &mut edge.pixels[3..] {
            *pixel = Color32::WHITE;
        }
        let result = gaussian_blur(&edge, 2);
        assert!(result.pixels[2].r() > 0);
        assert!(result.pixels[3].r() < 255);
        assert!(result.pixels.iter().all(|pixel| pixel.a() == 255));
    }
}
