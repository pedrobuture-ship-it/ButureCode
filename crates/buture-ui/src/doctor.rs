//! Observed diagnostics with explicit review before any supported repair.
use crate::Theme;
use buture_doctor::{Report, Severity, Solution};
use egui::{RichText, Ui};
pub enum Action {
    Scan,
    Fix(usize),
}
pub fn show(
    ui: &mut Ui,
    theme: &Theme,
    report: Option<&Report>,
    scanning: bool,
    selected: &mut Option<usize>,
) -> Option<Action> {
    ui.heading("Buture Doctor");
    let mut action = None;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!scanning, egui::Button::new("Scan environment"))
            .clicked()
        {
            action = Some(Action::Scan);
        }
        if ui
            .add_enabled(
                selected.is_some() && report.is_some(),
                egui::Button::new("Fix Selected…"),
            )
            .clicked()
        {
            action = selected.map(Action::Fix);
        }
        if scanning {
            ui.spinner();
        }
    });
    if let Some(report) = report {
        ui.label(RichText::new(format!("Environment: {}", report.status.label())).strong());
        ui.label(&report.scope);
        egui::ScrollArea::vertical().id_salt("doctor-diagnostics").show(ui,|ui|{
            let mut category="";
            for (index,d) in report.diagnostics.iter().enumerate(){
                if d.category!=category{ui.add_space(theme.spacing.lg);ui.heading(&d.category);ui.separator();category=&d.category;}
                let color=match d.severity{Severity::Info=>theme.text.secondary,Severity::Warning=>theme.semantic.warning,Severity::Error=>theme.semantic.error};
                ui.horizontal(|ui|{
                    if ui.selectable_label(*selected==Some(index),RichText::new(&d.title).color(color)).clicked(){*selected=Some(index);}
                    ui.label(&d.description);
                });
                if *selected==Some(index){ui.indent(index,|ui|{ui.label(format!("Cause: {}",d.cause));ui.label(match &d.solution{Solution::Manual(s)=>s.clone(),Solution::InstallTool(tool)=>format!("Review assisted installation for {tool} if supported on this host.")});});}
            }
        });
    } else {
        ui.label("No current report. Run a scan to inspect this environment.");
    }
    action
}
