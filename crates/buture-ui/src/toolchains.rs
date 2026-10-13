//! Toolchain manager renders observed states only.
use buture_toolchains::engine::{ProbeState, Toolchain};
use egui::Ui;
pub fn summary(reports: &[Toolchain]) -> String {
    let mut text = String::new();
    for report in reports {
        let ready = report
            .tools
            .iter()
            .filter(|t| matches!(t.state, ProbeState::Ready(_)))
            .count();
        let missing = report
            .tools
            .iter()
            .filter(|t| t.state == ProbeState::Missing)
            .count();
        let failed = report.tools.len() - ready - missing;
        text.push_str(&format!(
            "{}: {ready} verified · {missing} missing · {failed} probe failures\n",
            report.language
        ));
    }
    if text.is_empty() {
        text.push_str(
            "No provider results. Supported scans: Rust, C/C++, Python, Java, Node.js, Go.",
        );
    }
    text
}
pub fn show(ui: &mut Ui, reports: &[Toolchain], scanning: bool) -> bool {
    ui.heading("Development / Toolchains");
    ui.label("Versions and locations discovered on this host.");
    let scan = ui
        .add_enabled(!scanning, egui::Button::new("Scan tools"))
        .clicked();
    if scanning {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Scanning in background…");
        });
    }
    if reports.is_empty() {
        ui.label("Run a scan to inspect the development environment.");
    }
    egui::ScrollArea::vertical()
        .id_salt("toolchain-manager")
        .show(ui, |ui| {
            for report in reports {
                ui.add_space(16.0);
                ui.heading(&report.language);
                ui.separator();
                for (index, tool) in report.tools.iter().enumerate() {
                    let state = match &tool.state {
                        ProbeState::Missing => "Missing".into(),
                        ProbeState::Ready(version) => version.clone(),
                        ProbeState::Failed(_) => "Detected · version unavailable".into(),
                    };
                    ui.push_id((&report.language, index), |ui| {
                        ui.collapsing(format!("{}  ·  {state}", tool.name), |ui| {
                            ui.label(format!("Role: {:?}", tool.role));
                            if let Some(path) = &tool.executable {
                                ui.label(path.display().to_string());
                            }
                            if let ProbeState::Failed(reason) = &tool.state {
                                ui.label(reason);
                            }
                        });
                    });
                }
                if !report.environments.is_empty() {
                    ui.collapsing("Environments / installations", |ui| {
                        for p in &report.environments {
                            ui.label(p.display().to_string());
                        }
                    });
                }
            }
        });
    scan
}
