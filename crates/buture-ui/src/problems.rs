//! Problems filters run in a coalescing worker over shared diagnostic snapshots.
use crate::Theme;
use buture_lsp::features::{Diagnostic, Location};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
#[derive(Debug, Clone, PartialEq, Eq)]
struct Filter {
    errors: bool,
    warnings: bool,
    info: bool,
    query: String,
}
impl Default for Filter {
    fn default() -> Self {
        Self {
            errors: true,
            warnings: true,
            info: true,
            query: String::new(),
        }
    }
}
#[derive(Default, Debug, PartialEq, Eq)]
struct Counts {
    errors: usize,
    warnings: usize,
    info: usize,
}
struct Item {
    items: Arc<Vec<Diagnostic>>,
    index: usize,
}
struct Request {
    generation: u64,
    data: Vec<Arc<Vec<Diagnostic>>>,
    filter: Filter,
}
struct Results {
    generation: u64,
    items: Vec<Item>,
    counts: Counts,
}
struct Worker {
    sender: mpsc::SyncSender<Request>,
    events: mpsc::Receiver<Results>,
    latest: Arc<AtomicU64>,
}
impl Default for Worker {
    fn default() -> Self {
        let (sender, rx) = mpsc::sync_channel::<Request>(2);
        let (tx, events) = mpsc::sync_channel(2);
        let latest = Arc::new(AtomicU64::new(0));
        let current = latest.clone();
        std::thread::spawn(move || {
            while let Ok(mut request) = rx.recv() {
                while let Ok(newer) = rx.try_recv() {
                    request = newer;
                }
                if let Some((items, counts)) = filter(&request.data, &request.filter, || {
                    current.load(Ordering::Relaxed) != request.generation
                }) {
                    let _ = tx.try_send(Results {
                        generation: request.generation,
                        items,
                        counts,
                    });
                }
            }
        });
        Self {
            sender,
            events,
            latest,
        }
    }
}
fn filter(
    data: &[Arc<Vec<Diagnostic>>],
    filter: &Filter,
    cancel: impl Fn() -> bool,
) -> Option<(Vec<Item>, Counts)> {
    let query = filter.query.to_lowercase();
    let mut counts = Counts::default();
    let mut items = Vec::new();
    for record in data {
        for (index, item) in record.iter().enumerate() {
            if cancel() {
                return None;
            }
            let visible = match item.severity {
                1 => {
                    counts.errors += 1;
                    filter.errors
                }
                2 => {
                    counts.warnings += 1;
                    filter.warnings
                }
                _ => {
                    counts.info += 1;
                    filter.info
                }
            };
            if visible
                && (query.is_empty()
                    || item.message.to_lowercase().contains(&query)
                    || item.source.to_lowercase().contains(&query)
                    || item.path.to_string_lossy().to_lowercase().contains(&query))
            {
                items.push(Item {
                    items: record.clone(),
                    index,
                });
            }
        }
    }
    Some((items, counts))
}
#[derive(Default)]
pub struct Problems {
    filter: Filter,
    worker: Worker,
    signature: Option<(u64, Filter)>,
    generation: u64,
    returned: u64,
    results: Vec<Item>,
    counts: Counts,
}
impl Problems {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        revision: u64,
        data: Vec<Arc<Vec<Diagnostic>>>,
    ) -> Option<Location> {
        ui.horizontal(|ui| {
            ui.heading("Problems");
            ui.checkbox(
                &mut self.filter.errors,
                format!("Errors {}", self.counts.errors),
            );
            ui.checkbox(
                &mut self.filter.warnings,
                format!("Warnings {}", self.counts.warnings),
            );
            ui.checkbox(
                &mut self.filter.info,
                format!("Info/hints {}", self.counts.info),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.filter.query)
                    .hint_text("Filter file, message or source")
                    .char_limit(256)
                    .desired_width(240.0),
            );
        });
        let signature = (revision, self.filter.clone());
        if self.signature.as_ref() != Some(&signature) {
            let generation = self.generation + 1;
            self.worker.latest.store(generation, Ordering::Relaxed);
            if self
                .worker
                .sender
                .try_send(Request {
                    generation,
                    data,
                    filter: self.filter.clone(),
                })
                .is_ok()
            {
                self.generation = generation;
                self.signature = Some(signature);
                self.results.clear();
            }
        }
        while let Ok(result) = self.worker.events.try_recv() {
            if result.generation == self.generation {
                self.results = result.items;
                self.counts = result.counts;
                self.returned = result.generation;
            }
        }
        if self.returned != self.generation {
            ui.spinner();
            ui.label("Filtering diagnostics…");
        } else if self.results.is_empty() {
            ui.label(
                if self.counts.errors + self.counts.warnings + self.counts.info == 0 {
                    "No diagnostics received. Start a language server to collect them."
                } else {
                    "No diagnostics match this filter."
                },
            );
        }
        let mut selected = None;
        egui::ScrollArea::vertical()
            .id_salt("problem-results")
            .show_rows(ui, 28.0, self.results.len(), |ui, rows| {
                for row in rows {
                    let item = &self.results[row].items[self.results[row].index];
                    let color = match item.severity {
                        1 => theme.semantic.error,
                        2 => theme.semantic.warning,
                        _ => theme.semantic.info,
                    };
                    let line = item.range.start["line"].as_u64().unwrap_or(0) + 1;
                    let column = item.range.start["character"].as_u64().unwrap_or(0) + 1;
                    let message = item.message.chars().take(256).collect::<String>();
                    let label = format!(
                        "{}:{line}:{column} · {} · {message}",
                        item.path.display(),
                        item.source
                    );
                    let response = ui.add(
                        egui::Button::new(egui::RichText::new(label).color(color)).frame(false),
                    );
                    if response.clicked() {
                        selected = Some(Location {
                            path: item.path.clone(),
                            range: Some(item.range.clone()),
                        });
                    }
                    response.on_hover_text(&item.message);
                }
            });
        selected
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_keep_real_severity_source_and_locations() {
        let data = Arc::new(buture_lsp::features::diagnostics(
            &serde_json::json!({"uri":"file:///fixture.rs","diagnostics":[{"range":{"start":{"line":0,"character":2},"end":{"line":0,"character":3}},"severity":1,"message":"bad token","source":"parser"},{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":1}},"severity":2,"message":"unused","source":"lint"}]}),
        ));
        let selection = Filter {
            errors: false,
            query: "lint".into(),
            ..Default::default()
        };
        let (items, counts) =
            filter(std::slice::from_ref(&data), &selection, || false).expect("filter");
        assert_eq!(
            counts,
            Counts {
                errors: 1,
                warnings: 1,
                info: 0
            }
        );
        assert_eq!(items.len(), 1);
        let item = &items[0].items[items[0].index];
        assert_eq!(item.source, "lint");
        assert_eq!(item.range.start["line"], 1);
        assert!(filter(std::slice::from_ref(&data), &selection, || true).is_none());
    }
}
