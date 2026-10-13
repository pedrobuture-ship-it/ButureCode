//! Developer visual QA: capture real native framebuffer frames, then close.
//! cargo run -p buture-app --example capture_shell -- /tmp/buture-shell-preview
use buture_settings::VisualEffects;
use eframe::egui;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

const CASES: [(&str, VisualEffects, [f32; 2], bool, bool); 16] = [
    ("full", VisualEffects::Full, [1440.0, 900.0], true, true),
    (
        "reduced",
        VisualEffects::Reduced,
        [1440.0, 900.0],
        true,
        true,
    ),
    ("off", VisualEffects::Off, [1440.0, 900.0], true, true),
    ("compact", VisualEffects::Full, [960.0, 640.0], true, true),
    (
        "editor-only",
        VisualEffects::Full,
        [1440.0, 900.0],
        false,
        false,
    ),
    (
        "explorer",
        VisualEffects::Full,
        [1440.0, 900.0],
        false,
        false,
    ),
    ("editor", VisualEffects::Full, [1440.0, 900.0], false, false),
    ("tabs", VisualEffects::Full, [1440.0, 900.0], false, false),
    ("split", VisualEffects::Full, [1440.0, 900.0], false, false),
    ("search", VisualEffects::Full, [1440.0, 900.0], false, false),
    (
        "palette",
        VisualEffects::Full,
        [1440.0, 900.0],
        false,
        false,
    ),
    (
        "overview",
        VisualEffects::Full,
        [1440.0, 900.0],
        false,
        false,
    ),
    ("doctor", VisualEffects::Full, [1440.0, 900.0], false, false),
    (
        "toolchains",
        VisualEffects::Full,
        [1440.0, 900.0],
        false,
        false,
    ),
    (
        "installation-review",
        VisualEffects::Full,
        [1440.0, 900.0],
        false,
        false,
    ),
    (
        "onboarding",
        VisualEffects::Full,
        [1440.0, 900.0],
        false,
        false,
    ),
];

fn main() -> eframe::Result {
    let directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("buture-shell-preview"));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Buture Code — visual QA")
            .with_inner_size(CASES[0].2)
            .with_fullscreen(true)
            .with_decorations(false)
            .with_icon(egui::IconData::default()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Buture Code",
        options,
        Box::new(move |creation| {
            Ok(Box::new(Capture {
                shell: buture_ui::IdeShell::new(&creation.egui_ctx),
                directory,
                index: 0,
                requested: false,
                since_change: Instant::now(),
                editing_only: std::env::args().nth(2).as_deref() == Some("editing"),
            }))
        }),
    )
}

struct Capture {
    shell: buture_ui::IdeShell,
    directory: PathBuf,
    index: usize,
    requested: bool,
    since_change: Instant,
    editing_only: bool,
}

impl eframe::App for Capture {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.index >= CASES.len() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot {
            let path = self.directory.join(format!("{}.png", CASES[self.index].0));
            if let Err(error) = save_png(&path, &image) {
                eprintln!("Capture failed: {error}");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            println!(
                "Captured {} ({}×{})",
                path.display(),
                image.size[0],
                image.size[1]
            );
            self.index += 1;
            if self.index == CASES.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(CASES[self.index].2.into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                CASES[self.index].0 != "compact",
            ));
            if CASES[self.index].0 == "explorer" {
                self.shell
                    .open_workspace(std::env::current_dir().unwrap_or_default());
            }
            if CASES[self.index].0 == "editor"
                && let Ok(root) = std::env::current_dir()
            {
                self.shell
                    .open_file(root.join("crates/buture-app/src/main.rs"));
            }
            if CASES[self.index].0 == "tabs"
                && let Ok(root) = std::env::current_dir()
            {
                self.shell.open_file(root.join("Cargo.toml"));
            }
            if CASES[self.index].0 == "split" {
                self.shell.split_editor(false);
            }
            if CASES[self.index].0 == "palette" {
                self.shell.open_palette();
            }
            if self.editing_only && CASES[self.index].0 == "overview" {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            if CASES[self.index].0 == "search" {
                self.shell.preview_search("pub");
            }
            if CASES[self.index].0 == "overview" {
                self.shell.open_project_overview();
            }
            if CASES[self.index].0 == "doctor" {
                self.shell.open_doctor();
            }
            if CASES[self.index].0 == "toolchains" {
                self.shell.open_toolchain_manager();
            }
            if CASES[self.index].0 == "installation-review" {
                self.shell.review_missing_tool();
            }
            if CASES[self.index].0 == "onboarding" {
                self.shell.preview_onboarding();
            }
            self.requested = false;
            self.since_change = Instant::now();
        }
        let (_, effects, _, inspector, panel) = CASES[self.index];
        self.shell.effects = effects;
        self.shell.show_inspector = inspector;
        self.shell.show_bottom_panel = panel;
        self.shell.show(ctx);
        if !self.requested
            && !self.shell.environment_scan_pending()
            && self.since_change.elapsed() >= Duration::from_millis(800)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn save_png(
    path: &std::path::Path,
    image: &egui::ColorImage,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, image.size[0] as u32, image.size[1] as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let data: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|pixel| pixel.to_array())
        .collect();
    encoder.write_header()?.write_image_data(&data)?;
    Ok(())
}
