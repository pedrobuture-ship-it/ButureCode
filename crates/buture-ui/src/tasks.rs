//! Task selection and concrete command review are separate from execution consent.
use crate::Theme;
use buture_tasks::{Event, Execution, Plan, Task};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::mpsc,
};
#[derive(Default)]
pub struct Tasks {
    tasks: Vec<Task>,
    root: Option<PathBuf>,
    receiver: Option<mpsc::Receiver<Vec<Task>>>,
    plan: Option<Plan>,
    consent: bool,
    execution: Option<Execution>,
    log: VecDeque<String>,
    state: String,
    error: Option<String>,
}
impl Tasks {
    pub fn detect(&mut self, root: PathBuf, project: buture_project::Detection) {
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.root = Some(root);
        self.tasks.clear();
        self.plan = None;
        std::thread::spawn(move || {
            let _ = tx.send(buture_tasks::detect(&project));
        });
    }
    pub fn busy(&self) -> bool {
        self.receiver.is_some()
    }
    pub fn running(&self) -> bool {
        self.execution.is_some()
    }
    pub fn poll(&mut self) {
        if let Some(rx) = &self.receiver
            && let Ok(tasks) = rx.try_recv()
        {
            self.tasks = tasks;
            self.receiver = None;
        }
        let mut finished = false;
        if let Some(execution) = &self.execution {
            for event in execution.events.try_iter().take(256) {
                match event {
                    Event::Started(label) => self.state = format!("Running {label}"),
                    Event::Output { stderr, text } => {
                        self.log
                            .push_back(format!("{}{text}", if stderr { "stderr: " } else { "" }));
                        while self.log.len() > 256 {
                            self.log.pop_front();
                        }
                    }
                    Event::Finished {
                        result,
                        dropped_chunks,
                    } => {
                        self.state = match result {
                            Ok(()) => "Completed".into(),
                            Err(e) => e.to_string(),
                        };
                        if dropped_chunks > 0 {
                            self.log.push_back(format!(
                                "{dropped_chunks} output chunks omitted by queue limit"
                            ));
                        }
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.execution = None;
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme, root: Option<&Path>) {
        self.poll();
        ui.heading("Tasks");
        ui.label("Review the command before trusting project execution. Build scripts and package scripts can execute project code.");
        if root != self.root.as_deref() {
            ui.label("Open or rescan a project to discover tasks.");
        }
        if self.receiver.is_some() {
            ui.spinner();
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::from(theme.semantic.warning), error);
        }
        let mut selected = None;
        egui::ScrollArea::vertical().max_height(240.0).show_rows(
            ui,
            24.0,
            self.tasks.len(),
            |ui, rows| {
                for index in rows {
                    let task = &self.tasks[index];
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.running(), egui::Button::new(&task.label))
                            .clicked()
                        {
                            selected = Some(task.id.clone());
                        }
                        ui.add(egui::Label::new(task.cwd.display().to_string()).truncate());
                    });
                }
            },
        );
        if let Some(selected) = selected
            && let Some(root) = root
        {
            match Plan::review(&selected, &self.tasks, root) {
                Ok(plan) => {
                    self.plan = Some(plan);
                    self.consent = false;
                    self.error = None;
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        if let Some(plan) = self.plan.clone() {
            ui.separator();
            ui.heading("Review execution");
            for task in &plan.tasks {
                ui.label(format!("{} {:?}", task.command, task.args));
                ui.label(format!(
                    "cwd: {} · env keys: {}",
                    task.cwd.display(),
                    task.env.keys().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
            ui.checkbox(
                &mut self.consent,
                "I trust this workspace and reviewed these commands",
            );
            if ui
                .add_enabled(
                    self.consent && !self.running(),
                    egui::Button::new("Execute reviewed tasks"),
                )
                .clicked()
            {
                self.log.clear();
                self.state = "Starting…".into();
                self.execution = Some(plan.confirm().execute());
                self.plan = None;
            }
            if ui.button("Cancel review").clicked() {
                self.plan = None;
            }
        }
        ui.separator();
        ui.label(&self.state);
        if let Some(execution) = &self.execution
            && ui.button("Cancel task (direct child)").clicked()
        {
            execution.cancel();
        }
        egui::ScrollArea::vertical()
            .id_salt("task-output")
            .show(ui, |ui| {
                for text in &self.log {
                    ui.label(egui::RichText::new(text).monospace());
                }
            });
    }
}
