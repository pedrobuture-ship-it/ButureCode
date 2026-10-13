//! Native IDE shell, workspace navigation and development environment workflows.
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
    active: usize,
    sidebar_width: f32,
    explorer: crate::explorer::ExplorerView,
    workspace: buture_workspace::Workspace,
    root: Option<std::path::PathBuf>,
    branch: Option<String>,
    folder_input: String,
    open_folder: bool,
    file_menu: Option<std::path::PathBuf>,
    file_name: String,
    preview: Option<(std::path::PathBuf, String, Vec<usize>)>,
    message: String,
    preferences: Option<buture_settings::Preferences>,
    preferences_loaded: bool,
    settings: buture_settings::Settings,
    onboarding: crate::onboarding::Onboarding,
    host: Option<buture_toolchains::Host>,
    host_receiver: Option<
        std::sync::mpsc::Receiver<(
            buture_toolchains::Host,
            buture_toolchains::install::SetupHost,
        )>,
    >,
    scan_summary: Option<String>,
    pending_scan: bool,
    scan_languages: Vec<String>,
    preview_first_run: bool,
    scan: Option<buture_toolchains::engine::Scan>,
    reports: Vec<buture_toolchains::engine::Toolchain>,
    show_toolchains: bool,
    doctor: Option<buture_doctor::Report>,
    doctor_scan: Option<buture_doctor::Scan>,
    doctor_requested: bool,
    show_doctor: bool,
    doctor_selected: Option<usize>,
    manual_fix: Option<(String, String)>,
    setup: Option<buture_toolchains::install::SetupHost>,
    install_plan: Option<buture_toolchains::install::Plan>,
    install_confirmed: bool,
    installation: Option<buture_toolchains::install::Installation>,
    install_log: std::collections::VecDeque<String>,
}

impl IdeShell {
    pub fn new(ctx: &Context) -> Self {
        let theme = Theme::dark();
        configure(ctx, &theme);
        Self {
            backdrop: Backdrop::new(ctx, &theme),
            active: 0,
            sidebar_width: 210.0,
            explorer: Default::default(),
            workspace: Default::default(),
            root: None,
            branch: None,
            folder_input: String::new(),
            open_folder: false,
            file_menu: None,
            file_name: String::new(),
            preview: None,
            message: String::new(),
            preferences: None,
            preferences_loaded: false,
            settings: Default::default(),
            onboarding: Default::default(),
            host: None,
            host_receiver: None,
            scan_summary: None,
            pending_scan: false,
            scan_languages: Vec::new(),
            preview_first_run: false,
            scan: None,
            reports: Vec::new(),
            show_toolchains: false,
            setup: None,
            doctor: None,
            doctor_scan: None,
            doctor_requested: false,
            show_doctor: false,
            doctor_selected: None,
            manual_fix: None,
            install_plan: None,
            install_confirmed: false,
            installation: None,
            install_log: Default::default(),
            theme,
            effects: VisualEffects::Full,
            show_inspector: true,
            show_bottom_panel: true,
        }
    }

    pub fn start_preferences(&mut self) {
        self.preferences = Some(buture_settings::Preferences::start());
        self.initialize_environment();
    }
    fn initialize_environment(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.host_receiver = Some(rx);
        std::thread::spawn(move || {
            let host = buture_toolchains::Host::detect();
            let setup = buture_toolchains::install::SetupHost::detect(&host);
            let _ = tx.send((host, setup));
        });
    }
    pub fn open_workspace(&self, path: std::path::PathBuf) {
        self.workspace.send(buture_workspace::Request::Open(path));
    }
    pub fn open_doctor(&mut self) {
        self.show_doctor = true;
        self.show_toolchains = false;
        self.doctor_requested = true;
        self.doctor = None;
        self.doctor_selected = None;
        if self.host.is_none() {
            self.initialize_environment();
            self.pending_scan = true;
        } else {
            self.start_scan(self.required_languages());
        }
    }
    fn required_languages(&self) -> Vec<String> {
        self.settings
            .get("Languages.selected")
            .unwrap_or("")
            .split(',')
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    }
    pub fn open_toolchain_manager(&mut self) {
        self.show_doctor = false;
        self.active = 7;
        self.show_toolchains = true;
        self.pending_scan = true;
        if self.host.is_none() && self.host_receiver.is_none() {
            self.initialize_environment();
        }
    }
    /// Open review for an observed missing tool without confirming execution.
    pub fn review_missing_tool(&mut self) -> bool {
        self.install_plan = self.setup.as_ref().and_then(|h| {
            self.reports
                .iter()
                .flat_map(|r| r.tools.iter())
                .filter(|t| t.state == buture_toolchains::engine::ProbeState::Missing)
                .find_map(|t| buture_toolchains::install::Plan::for_tool(&t.name, h))
        });
        self.install_confirmed = false;
        self.install_plan.is_some()
    }
    pub fn environment_scan_pending(&self) -> bool {
        self.pending_scan || self.scan.is_some() || self.doctor_scan.is_some()
    }
    /// Preview first run without reading or writing personal preferences.
    pub fn preview_onboarding(&mut self) {
        self.install_plan = None;
        self.preview_first_run = true;
        self.initialize_environment();
    }
    fn save_preference(&self, key: &str, value: &str) {
        if let Some(p) = &self.preferences {
            let _ = p.requests.send(buture_settings::Request::SetUser {
                key: key.into(),
                value: value.into(),
            });
        }
    }
    fn start_scan(&mut self, languages: Vec<String>) {
        if self.scan.is_none()
            && let Some(host) = &self.host
        {
            self.scan_languages = languages.clone();
            self.doctor = None;
            self.reports.clear();
            self.scan_summary = None;
            self.onboarding.scanned = false;
            self.scan = Some(buture_toolchains::engine::Scan::start(
                host.clone(),
                languages,
            ));
        }
    }
    pub fn show(&mut self, ctx: &Context) {
        if let Some(scan) = &self.doctor_scan
            && let Ok(report) = scan.events.try_recv()
        {
            self.doctor = Some(report);
            self.doctor_scan = None;
            self.doctor_requested = false;
        }

        let mut complete = false;
        if let Some(scan) = &self.scan {
            while let Ok(event) = scan.events.try_recv() {
                match event {
                    buture_toolchains::engine::ScanEvent::Result(report) => {
                        self.reports.push(report)
                    }
                    buture_toolchains::engine::ScanEvent::Complete => complete = true,
                }
            }
        }
        if complete {
            self.scan = None;
            self.reports.sort_by(|a, b| a.language.cmp(&b.language));
            self.scan_summary = Some(crate::toolchains::summary(&self.reports));
            if self.doctor_requested
                && let Some(host) = &self.host
            {
                self.doctor_scan = Some(buture_doctor::Scan::start(
                    host.clone(),
                    self.reports.clone(),
                    self.root.clone(),
                    self.required_languages(),
                ));
            }
        }

        if let Some(p) = &self.preferences {
            while let Ok(result) = p.events.try_recv() {
                match result {
                    Ok(settings) => {
                        self.preferences_loaded = true;
                        self.effects = settings.effects();
                        self.show_inspector = settings.get("Appearance.inspector") == Some("true");
                        self.show_bottom_panel = settings.get("Appearance.panel") == Some("true");
                        self.settings = settings;
                    }
                    Err(e) => {
                        self.preferences_loaded = true;
                        self.message = e.to_string();
                    }
                }
            }
        }

        if let Some(rx) = &self.host_receiver
            && let Ok((host, setup)) = rx.try_recv()
        {
            self.host = Some(host);
            self.setup = Some(setup);
        }
        if self.pending_scan && self.host.is_some() {
            self.pending_scan = false;
            self.start_scan(if self.doctor_requested {
                self.required_languages()
            } else {
                Vec::new()
            });
        }
        while let Ok(event) = self.workspace.events.try_recv() {
            match event {
                buture_workspace::Event::Tree {
                    root,
                    entries,
                    branch,
                } => {
                    if self.root.as_ref() != Some(&root)
                        && let Some(p) = &self.preferences
                    {
                        let _ = p
                            .requests
                            .send(buture_settings::Request::LoadWorkspace(root.clone()));
                    }
                    if self.root.as_ref() != Some(&root) {
                        self.preview = None;
                        self.explorer.selected = None;
                    }
                    self.branch = branch;
                    self.root = Some(root);
                    self.explorer.rows = entries
                        .into_iter()
                        .map(|e| crate::explorer::TreeRow {
                            path: e.path,
                            directory: e.directory,
                            depth: e.depth,
                            expanded: e.expanded,
                        })
                        .collect();
                }
                buture_workspace::Event::Preview {
                    path,
                    text,
                    line_offsets,
                } => self.preview = Some((path, text, line_offsets)),
                buture_workspace::Event::Error(error) => self.message = error,
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.open_folder = true;
        }
        self.workspace_dialogs(ctx);
        self.install_dialogs(ctx);
        if self.preview_first_run
            || (self.preferences.is_some()
                && self.preferences_loaded
                && self.settings.get("Development.onboardingComplete") != Some("true"))
        {
            let languages_match = self.scan_languages == self.onboarding.languages;
            let action = egui::Window::new("First run")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .frame(egui::Frame::NONE)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    GlassSurface::new(GlassVariant::Modal)
                        .show(ui, &self.theme, self.effects, &mut self.backdrop, |ui| {
                            ui.set_min_width(450.0);
                            ui.set_min_height(220.0);
                            self.onboarding.show(
                                ui,
                                self.host.as_ref(),
                                self.scan_summary.as_deref().filter(|_| languages_match),
                                self.scan.is_some(),
                            )
                        })
                        .inner
                })
                .and_then(|r| r.inner)
                .flatten();
            if let Some(action) = action {
                match action {
                    crate::onboarding::Action::Scan => {
                        self.start_scan(self.onboarding.languages.clone())
                    }
                    crate::onboarding::Action::Complete => {
                        self.preview_first_run = false;
                        self.save_preference(
                            "Languages.selected",
                            &self.onboarding.languages.join(","),
                        );
                        self.save_preference("Development.onboardingComplete", "true");
                    }
                }
            }
        }
        let theme = self.theme;
        let mut layout = ShellLayout::calculate(
            ctx.content_rect(),
            &theme,
            self.show_inspector,
            self.show_bottom_panel,
        );
        let shift = self.sidebar_width - layout.sidebar.width();
        layout.sidebar.max.x += shift;
        layout.editor.min.x += shift;
        if let Some(bottom) = &mut layout.bottom {
            bottom.min.x += shift;
        }
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
                    &mut self.active,
                );
                let handle = Rect::from_min_max(
                    pos2(layout.sidebar.right() - 3.0, layout.sidebar.top()),
                    pos2(layout.sidebar.right() + 3.0, layout.sidebar.bottom()),
                );
                let resize = ui.interact(handle, ui.id().with("sidebar-resize"), Sense::drag());
                if resize.dragged() {
                    self.sidebar_width = (self.sidebar_width + resize.drag_delta().x)
                        .clamp(150.0, (layout.outer.width() * 0.28).min(360.0));
                }
                resize.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                let mut side = region(ui, "sidebar", layout.sidebar.shrink(theme.spacing.md));
                if self.active == 0 {
                    side.horizontal(|ui| {
                        ui.heading("Explorer");
                        if ui.small_button("Open…").clicked() {
                            self.open_folder = true;
                        }
                    });
                    side.horizontal(|ui| {
                        if ui.small_button("Refresh").clicked() {
                            self.workspace.send(buture_workspace::Request::Refresh);
                        }
                        if ui.small_button("Collapse").clicked() {
                            self.workspace.send(buture_workspace::Request::Collapse);
                        }
                    });
                    side.horizontal(|ui|{if ui.small_button("Doctor").clicked(){self.open_doctor();}
if self.show_doctor && ui.small_button("Editor").clicked(){self.show_doctor=false;}});
                    side.separator();
                    if self.explorer.rows.is_empty() {
                        side.label("No folder open");
                    }
                    if let Some(action) = self.explorer.show(&mut side) {
                        use crate::explorer::TreeAction;
                        match action {
                            TreeAction::Toggle(path) => {
                                self.workspace.send(buture_workspace::Request::Toggle(path))
                            }
                            TreeAction::Open(path) => {
                                self.workspace.send(buture_workspace::Request::Read(path))
                            }
                            TreeAction::Menu(path) => {
                                self.file_menu = Some(path);
                                self.file_name.clear();
                            }
                        }
                    }
                } else if self.active == 7 {
                    side.heading("Settings");
                    egui::ScrollArea::vertical().show(&mut side, |ui| {
                        for category in buture_settings::CATEGORIES {
                            ui.collapsing(category, |ui| {
                                if category == "Appearance" {
                                    for effects in VisualEffects::ALL {
                                        if ui
                                            .selectable_value(
                                                &mut self.effects,
                                                effects,
                                                effects.label(),
                                            )
                                            .clicked()
                                        {
                                            self.save_preference(
                                                "Appearance.effects",
                                                effects.label(),
                                            );
                                        }
                                    }
                                } else if category == "Development" || category == "Toolchains" {
                                    if ui.button("Toolchains").clicked() {
                                        self.show_toolchains = true;
                                    }
                                } else {
                                    ui.label("Preferences will be added with this feature.");
                                }
                            });
                        }
                    });
                } else {
                    sidebar(&mut side, &theme, self.active);
                }
                let mut center = region(ui, "editor", layout.editor.shrink(theme.spacing.md));
                if self.show_doctor {
                    if let Some(action)=crate::doctor::show(&mut center,&theme,self.doctor.as_ref(),self.scan.is_some() || self.doctor_scan.is_some(),&mut self.doctor_selected){
                        match action {
                            crate::doctor::Action::Scan=>self.open_doctor(),
                            crate::doctor::Action::Fix(index)=>{
                                if let Some(d)=self.doctor.as_ref().and_then(|r|r.diagnostics.get(index)){
                                    match &d.solution{
                                        buture_doctor::Solution::InstallTool(tool)=>{
                                            self.install_plan=self.setup.as_ref().and_then(|h|buture_toolchains::install::Plan::for_tool(tool,h));self.install_confirmed=false;
                                            if self.install_plan.is_none(){self.manual_fix=Some((d.title.clone(),"No assisted installer is supported for this tool and host. Review its official installation instructions.".into()));}
                                        }
                                        buture_doctor::Solution::Manual(steps)=>self.manual_fix=Some((d.title.clone(),steps.clone())),
                                    }
                                }
                            }
                        }
                    }
                } else if self.active == 7 && self.show_toolchains {
                    if let Some(action) = crate::toolchains::show(
                        &mut center,
                        &self.reports,
                        self.scan.is_some() || self.installation.is_some(),
                        self.setup.as_ref(),
                    ) {
                        match action {
                            crate::toolchains::Action::Scan => self.start_scan(Vec::new()),
                            crate::toolchains::Action::Install(tool) => {
                                self.install_plan = self.setup.as_ref().and_then(|h| {
                                    buture_toolchains::install::Plan::for_tool(&tool, h)
                                });
                                self.install_confirmed = false;
                            }
                        }
                    }
                } else if let Some((path, text, offsets)) = &self.preview {
                    center.label(path.display().to_string());
                    center.label("Read-only preview · editing belongs to prompt 25");
                    center.separator();
                    egui::ScrollArea::both().show_rows(
                        &mut center,
                        20.0,
                        offsets.len(),
                        |ui, range| {
                            for index in range {
                                let start = offsets[index];
                                let end = offsets.get(index + 1).copied().unwrap_or(text.len());
                                let line = text
                                    .get(start..end)
                                    .unwrap_or("")
                                    .trim_end_matches(['\r', '\n']);
                                ui.add(
                                    egui::Label::new(RichText::new(line).monospace())
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                );
                            }
                        },
                    );
                } else {
                    editor(&mut center, &theme);
                }
                if let Some(rect) = layout.inspector {
                    self.inspector(&mut region(ui, "inspector", rect.shrink(theme.spacing.xl)));
                }
                if let Some(rect) = layout.bottom {
                    output_panel(
                        &mut region(ui, "output", rect.shrink(theme.spacing.lg)),
                        &theme,
                        self.root.as_deref(),
                        &self.message,
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
                    self.root.as_deref(),
                    &self.message,
                );
            });
    }

    fn install_dialogs(&mut self, ctx: &Context) {
        if let Some((title, steps)) = self.manual_fix.clone() {
            egui::Window::new("Review Doctor fix")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.heading(title);
                    ui.label(steps);
                    ui.label(
                        "These are manual instructions. Doctor has not changed your environment.",
                    );
                    if ui.button("Close review").clicked() {
                        self.manual_fix = None;
                    }
                });
        }

        let mut finished = None;
        if let Some(installation) = &self.installation {
            while let Ok(event) = installation.events.try_recv() {
                match event {
                    buture_toolchains::install::InstallEvent::Output(text) => {
                        self.install_log.push_back(text);
                        while self.install_log.len() > 128 {
                            self.install_log.pop_front();
                        }
                    }
                    buture_toolchains::install::InstallEvent::Finished(result) => {
                        finished = Some(result)
                    }
                }
            }
        }
        if let Some(result) = finished {
            self.installation = None;
            self.message = match result {
                Ok(buture_toolchains::engine::ProbeState::Ready(version)) => {
                    format!("Installation verified: {version}")
                }
                Ok(state) => format!("Package manager completed; verification: {state:?}"),
                Err(error) => error,
            };
            self.install_log.push_back(self.message.clone());
            self.start_scan(Vec::new());
        }
        if let Some(plan) = self.install_plan.clone() {
            egui::Window::new("Review installation")
                .collapsible(false)
                .default_width(540.0)
                .show(ctx, |ui| {
                    ui.heading(format!("Missing tool: {}", plan.tool()));
                    ui.label(&plan.method);
                    ui.label(&plan.explanation);
                    ui.separator();
                    ui.label("Exact command");
                    ui.add(egui::Label::new(RichText::new(plan.command()).monospace()).wrap());
                    if ui.button("Copy command").clicked() {
                        ctx.copy_text(plan.command());
                    }
                    ui.checkbox(
                        &mut self.install_confirmed,
                        "I authorize this command and its package changes",
                    );
                    if ui
                        .add_enabled(
                            self.install_confirmed && self.installation.is_none(),
                            Button::new("Execute approved installation"),
                        )
                        .clicked()
                    {
                        self.install_log.clear();
                        self.installation = Some(plan.confirm().execute());
                        self.install_plan = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.install_plan = None;
                        self.install_confirmed = false;
                    }
                });
        }
        if self.installation.is_some() || !self.install_log.is_empty() {
            egui::Window::new("Installation output").default_width(600.0).show(ctx,|ui| {
                if self.installation.is_some(){ui.spinner();ui.label("Installation running. Closing this panel does not cancel package changes.");}
                egui::ScrollArea::vertical().max_height(300.0).show(ui,|ui|{for line in &self.install_log {ui.label(RichText::new(line).monospace());}});
                if self.installation.is_none() && ui.button("Close").clicked(){self.install_log.clear();}
            });
        }
    }

    fn workspace_dialogs(&mut self, ctx: &Context) {
        if self.open_folder {
            egui::Window::new("Open Folder")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Absolute path to your workspace");
                    ui.text_edit_singleline(&mut self.folder_input);
                    if ui.button("Open").clicked() {
                        self.workspace.send(buture_workspace::Request::Open(
                            self.folder_input.clone().into(),
                        ));
                        self.open_folder = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.open_folder = false;
                    }
                });
        }
        if let Some(path) = self.file_menu.clone() {
            egui::Window::new("File actions")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(path.display().to_string());
                    if ui.button("Copy path").clicked() {
                        ctx.copy_text(path.display().to_string());
                    }
                    if ui.button("Reveal in file manager").clicked() {
                        self.workspace
                            .send(buture_workspace::Request::Reveal(path.clone()));
                    }
                    ui.label("Name or copy destination inside the workspace");
                    ui.text_edit_singleline(&mut self.file_name);
                    for (label, op) in [
                        (
                            "New file",
                            buture_workspace::Operation::NewFile(self.file_name.clone()),
                        ),
                        (
                            "New folder",
                            buture_workspace::Operation::NewFolder(self.file_name.clone()),
                        ),
                        (
                            "Rename",
                            buture_workspace::Operation::Rename(self.file_name.clone()),
                        ),
                        (
                            "Duplicate",
                            buture_workspace::Operation::Duplicate(self.file_name.clone()),
                        ),
                        (
                            "Copy to path",
                            buture_workspace::Operation::CopyTo(self.file_name.clone().into()),
                        ),
                    ] {
                        if ui.button(label).clicked() {
                            self.workspace
                                .send(buture_workspace::Request::Mutate(path.clone(), op));
                            self.file_menu = None;
                        }
                    }
                    ui.separator();
                    ui.label("Delete permanently removes this entry and its contents.");
                    if ui.button("Confirm permanent deletion").clicked() {
                        self.workspace.send(buture_workspace::Request::Mutate(
                            path,
                            buture_workspace::Operation::Delete,
                        ));
                        self.file_menu = None;
                    }
                    if ui.button("Close").clicked() {
                        self.file_menu = None;
                    }
                });
        }
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
            let shortcut = if cfg!(target_os = "macos") {
                "⌘ K"
            } else {
                "Ctrl K"
            };
            let width = (ui.available_width() - 380.0).clamp(160.0, 440.0);
            ui.add_sized(
                [width, 30.0],
                Button::new(format!("Search files, symbols, commands...   {shortcut}")),
            )
            .on_hover_text("Command Center · available in a later milestone");
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
                ui.add_enabled(false, Button::new("Profile"));
                ui.label(self.branch.as_deref().unwrap_or("No branch"));
                if ui.button("Workspace").clicked() {
                    self.open_folder = true;
                }
                if ui
                    .toggle_value(&mut self.show_inspector, "Inspector")
                    .on_hover_text("Toggle right inspector")
                    .changed()
                {
                    self.save_preference(
                        "Appearance.inspector",
                        if self.show_inspector { "true" } else { "false" },
                    );
                }
                if ui
                    .toggle_value(&mut self.show_bottom_panel, "Panel")
                    .on_hover_text("Toggle output panel")
                    .changed()
                {
                    self.save_preference(
                        "Appearance.panel",
                        if self.show_bottom_panel {
                            "true"
                        } else {
                            "false"
                        },
                    );
                }
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
                if ui
                    .selectable_value(&mut self.effects, effects, effects.label())
                    .changed()
                {
                    self.save_preference("Appearance.effects", effects.label());
                }
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

fn activity_bar(ui: &mut Ui, theme: &Theme, active: &mut usize) {
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
            ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
        if response.clicked() {
            *active = index;
        }
        let hovered = response.hovered();
        response.on_hover_text(label);
        let selected = index == *active;
        if hovered && !selected {
            ui.painter().rect_filled(
                rect,
                theme.radius.small,
                Color32::from(theme.surface.elevated),
            );
        }
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
            ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
        if response.clicked() {
            *active = 7;
        }
        response.on_hover_text("Settings");
        Icon::Settings.paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(20.0)),
            theme.text.muted.into(),
        );
    });
}
fn sidebar(ui: &mut Ui, theme: &Theme, active: usize) {
    let title = [
        "Explorer",
        "Search",
        "Source Control",
        "Run & Debug",
        "Database",
        "Docker",
        "Hardware",
        "Settings",
    ][active];
    if active != 0 {
        ui.heading(title);
        ui.separator();
        ui.label("This workspace will be implemented in its dedicated milestone.");
        return;
    }
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
fn output_panel(ui: &mut Ui, theme: &Theme, root: Option<&std::path::Path>, message: &str) {
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
                RichText::new(
                    root.map(|p| format!("Workspace: {}", p.display()))
                        .unwrap_or_else(|| "No workspace open.".into()),
                )
                .monospace()
                .color(theme.text.muted),
            );
            if !message.is_empty() {
                ui.label(RichText::new(message).color(theme.text.secondary));
            }
        });
}
fn status_bar(ui: &mut Ui, theme: &Theme, root: Option<&std::path::Path>, message: &str) {
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
        let workspace = root
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "No workspace".into());
        caption(ui, &workspace, theme);
        if !message.is_empty() {
            ui.label(RichText::new("Output message").color(theme.text.secondary))
                .on_hover_text(message);
        }
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
