//! Lazy path-only index; fuzzy scoring and traversal stay off the UI thread.
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};
#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub relative: PathBuf,
    pub name: String,
}
pub enum Event {
    Built {
        root: PathBuf,
        files: usize,
        partial: bool,
    },
    Results {
        root: PathBuf,
        generation: u64,
        files: Vec<File>,
    },
    Failed {
        root: PathBuf,
        message: String,
    },
}
enum Request {
    Build {
        root: PathBuf,
        generation: u64,
    },
    Query {
        root: PathBuf,
        text: String,
        generation: u64,
    },
}
pub struct Index {
    pub events: mpsc::Receiver<Event>,
    sender: mpsc::SyncSender<Request>,
    build: Arc<AtomicU64>,
    query: Arc<AtomicU64>,
}
impl Default for Index {
    fn default() -> Self {
        let (sender, rx) = mpsc::sync_channel(8);
        let (tx, events) = mpsc::channel();
        let build = Arc::new(AtomicU64::new(0));
        let query = Arc::new(AtomicU64::new(0));
        let build_flag = build.clone();
        let query_flag = query.clone();
        std::thread::spawn(move || {
            let mut indexed_root = None;
            let mut cached: Vec<File> = Vec::new();
            while let Ok(request) = rx.recv() {
                match request {
                    Request::Build { root, generation } => {
                        let result = (|| {
                            let root = std::fs::canonicalize(&root).map_err(|e| e.to_string())?;
                            let mut files = Vec::new();
                            let mut partial = false;
                            let mut walk = ignore::WalkBuilder::new(&root);
                            walk.hidden(false)
                                .follow_links(false)
                                .require_git(false)
                                .filter_entry(|e| {
                                    !matches!(
                                        e.file_name().to_str(),
                                        Some(".git" | "target" | "node_modules" | ".venv" | ".aws")
                                    )
                                });
                            for (index, entry) in walk.build().enumerate() {
                                if build_flag.load(Ordering::Relaxed) != generation {
                                    return Ok(None);
                                }
                                if index >= 100_000 {
                                    partial = true;
                                    break;
                                }
                                let entry = match entry {
                                    Ok(e) => e,
                                    Err(_) => {
                                        partial = true;
                                        continue;
                                    }
                                };
                                if !entry.file_type().is_some_and(|t| t.is_file()) {
                                    continue;
                                }
                                files.push(File {
                                    path: entry.path().into(),
                                    relative: entry
                                        .path()
                                        .strip_prefix(&root)
                                        .map_err(|e| e.to_string())?
                                        .into(),
                                    name: entry.file_name().to_string_lossy().into_owned(),
                                });
                            }
                            files.sort_by(|a, b| a.relative.cmp(&b.relative));
                            Ok(Some((root, files, partial)))
                        })();
                        match result {
                            Ok(Some((root, files, partial))) => {
                                let count = files.len();
                                cached = files;
                                indexed_root = Some(root.clone());
                                if tx
                                    .send(Event::Built {
                                        root,
                                        files: count,
                                        partial,
                                    })
                                    .is_err()
                                {
                                    return;
                                }
                            }
                            Ok(None) => {}
                            Err(message) => {
                                let _ = tx.send(Event::Failed { root, message });
                            }
                        }
                    }
                    Request::Query {
                        root,
                        text,
                        generation,
                    } => {
                        if indexed_root.as_ref() != Some(&root)
                            || query_flag.load(Ordering::Relaxed) != generation
                        {
                            continue;
                        }
                        let mut scored = Vec::new();
                        let mut cancelled = false;
                        for (index, file) in cached.iter().enumerate() {
                            if index % 128 == 0 && query_flag.load(Ordering::Relaxed) != generation
                            {
                                cancelled = true;
                                break;
                            }
                            let score = buture_core::commands::fuzzy_score(&text, &file.name)
                                .map(|s| s + 1000)
                                .or_else(|| {
                                    buture_core::commands::fuzzy_score(
                                        &text,
                                        &file.relative.to_string_lossy(),
                                    )
                                });
                            if let Some(score) = score {
                                scored.push((index, score));
                            }
                        }
                        if cancelled {
                            continue;
                        }
                        scored.sort_by(|(a, sa), (b, sb)| {
                            sb.cmp(sa)
                                .then_with(|| cached[*a].relative.cmp(&cached[*b].relative))
                        });
                        let files = scored
                            .into_iter()
                            .take(200)
                            .map(|(i, _)| cached[i].clone())
                            .collect();
                        if tx
                            .send(Event::Results {
                                root,
                                generation,
                                files,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
        });
        Self {
            events,
            sender,
            build,
            query,
        }
    }
}
impl Index {
    pub fn build(&self, root: PathBuf) -> bool {
        let generation = self.build.fetch_add(1, Ordering::Relaxed) + 1;
        self.sender
            .try_send(Request::Build { root, generation })
            .is_ok()
    }
    pub fn query(&self, root: PathBuf, text: String, generation: u64) -> bool {
        self.query.store(generation, Ordering::Relaxed);
        self.sender
            .try_send(Request::Query {
                root,
                text,
                generation,
            })
            .is_ok()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn index_respects_ignores_and_returns_real_paths() {
        let root = std::env::temp_dir().join(format!("buture-index-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).expect("fixture");
        std::fs::write(root.join("src/main.rs"), "fn main(){}").expect("source");
        std::fs::write(root.join("hidden.rs"), "").expect("ignored");
        std::fs::write(root.join(".gitignore"), "hidden.rs\n").expect("ignore");
        let root = std::fs::canonicalize(root).expect("root");
        let index = Index::default();
        assert!(index.build(root.clone()));
        assert!(matches!(
            index.events.recv_timeout(std::time::Duration::from_secs(3)),
            Ok(Event::Built { .. })
        ));
        assert!(index.query(root.clone(), "mnrs".into(), 1));
        match index
            .events
            .recv_timeout(std::time::Duration::from_secs(3))
            .expect("results")
        {
            Event::Results { files, .. } => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].name, "main.rs");
                assert!(files[0].path.is_file());
            }
            _ => panic!("unexpected index event"),
        };
        drop(index);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
