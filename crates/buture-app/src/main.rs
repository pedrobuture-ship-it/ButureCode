//! Native application boundary. No workspace commands or network access.
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(buture_core::APP_NAME)
            .with_app_id("com.buture.code")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 640.0])
            .with_decorations(false)
            .with_icon(eframe::egui::IconData::default()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        buture_core::APP_NAME,
        options,
        Box::new(|creation| {
            let mut shell = buture_ui::IdeShell::new(&creation.egui_ctx);
            shell.start_preferences();
            Ok(Box::new(ButureApp { shell }))
        }),
    )
}

struct ButureApp {
    shell: buture_ui::IdeShell,
}

impl eframe::App for ButureApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shell.persist_session();
    }
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.shell.show(ctx);
    }
}
