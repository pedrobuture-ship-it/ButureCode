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
    run_only: bool,
    run_requested: bool,
    preferences: Option<buture_tasks::run::Preferences>,
    choices: std::collections::BTreeMap<String, String>,
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
    pub fn flush_preferences(&self) {
        if let Some(preferences) = &self.preferences {
            preferences.flush();
        }
    }
    pub fn configure_preferences(&mut self, path: PathBuf) {
        self.preferences = Some(buture_tasks::run::Preferences::start(path));
    }
    pub fn smart_run(&mut self) {
        self.run_only = true;
        self.run_requested = true;
    }
    fn prepare(&mut self, id: &str) {
        if let Some(root) = &self.root {
            match Plan::review(id, &self.tasks, root) {
                Ok(plan) => {
                    self.plan = Some(plan);
                    self.consent = false;
                    self.error = None;
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }
    }
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
        if let Some(preferences) = &self.preferences {
            while let Ok(event) = preferences.events.try_recv() {
                match event {
                    buture_tasks::run::Event::Loaded(values) => self.choices = values,
                    buture_tasks::run::Event::Failed(error) => self.error = Some(error.to_string()),
                }
            }
        }
        if let Some(rx) = &self.receiver
            && let Ok(tasks) = rx.try_recv()
        {
            self.tasks = tasks;
            self.receiver = None;
        }
        if self.run_requested && self.receiver.is_none() && self.root.is_some() {
            self.run_requested = false;
            let remembered = self
                .root
                .as_ref()
                .and_then(|root| self.choices.get(&root.to_string_lossy().to_string()))
                .map(String::as_str);
            match buture_tasks::run::choose(&self.tasks,remembered) { buture_tasks::run::Choice::Selected(id)=>self.prepare(&id), buture_tasks::run::Choice::Ambiguous(_)=>self.state="Multiple run targets: select the one to use".into(), buture_tasks::run::Choice::Unavailable=>self.state="No run entrypoint could be inferred. Select a configured task or add a project run target.".into() }
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
        ui.horizontal(|ui| {
            ui.heading(if self.run_only { "Smart Run" } else { "Tasks" });
            if ui.button("RUN").clicked() {
                self.smart_run();
            }
            ui.checkbox(&mut self.run_only, "Run targets only");
        });
        ui.label("Review the command before trusting project execution. Build scripts and package scripts can execute project code.");
        if root.is_none() || root != self.root.as_deref() {
            ui.label("Open or rescan a project to discover tasks.");
        }
        if self.receiver.is_some() {
            ui.spinner();
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::from(theme.semantic.warning), error);
        }
        let mut selected = None;
        let filtered: Vec<_> = self
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| !self.run_only || t.run)
            .map(|(i, _)| i)
            .collect();
        egui::ScrollArea::vertical().max_height(240.0).show_rows(
            ui,
            24.0,
            filtered.len(),
            |ui, rows| {
                for index in rows {
                    let task = &self.tasks[filtered[index]];
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
        if let Some(selected) = selected {
            self.prepare(&selected);
            if self.run_only
                && let (Some(preferences), Some(root)) = (&self.preferences, &self.root)
            {
                preferences.remember(root, selected);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn run_filter_renders_only_available_targets_without_indexing_hidden_tasks() {
        let mut tasks = Tasks::default();
        for run in [false, true, false] {
            tasks.tasks.push(Task {
                id: format!("task{}", tasks.tasks.len()),
                label: "fixture".into(),
                command: "echo".into(),
                args: vec![],
                env: Default::default(),
                cwd: std::env::temp_dir(),
                dependencies: vec![],
                run,
            });
        }
        tasks.run_only = true;
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| tasks.show(ui, &Theme::dark(), None));
        });
    }
}
