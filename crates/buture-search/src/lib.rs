//! Bounded, cancellable project search. Replacements are planned and explicitly confirmed.
use buture_editor::{Buffer, Edit, TextSnapshot};
use regex::{Regex, RegexBuilder};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};
const FILE_LIMIT: u64 = 16 * 1024 * 1024;
const RESULT_LIMIT: usize = 10_000;
#[derive(Debug, Clone, Default)]
pub struct Query {
    pub text: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}
#[derive(Debug)]
pub enum SearchError {
    Io(std::io::Error),
    Pattern(String),
    InvalidScope,
    TooLarge,
    ModifiedDocument,
    Buffer(buture_editor::EditorError),
}
impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Pattern(e) => write!(f, "Invalid search: {e}"),
            Self::InvalidScope => write!(
                f,
                "Search path escapes the workspace or traverses a symlink"
            ),
            Self::TooLarge => write!(f, "Search or replacement plan reached its size limit"),
            Self::ModifiedDocument => write!(
                f,
                "An open document has unsaved changes; save or close it before workspace replacement"
            ),
            Self::Buffer(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for SearchError {}
impl From<std::io::Error> for SearchError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<buture_editor::EditorError> for SearchError {
    fn from(e: buture_editor::EditorError) -> Self {
        Self::Buffer(e)
    }
}
impl Query {
    pub fn compile(&self) -> Result<Regex, SearchError> {
        if self.text.is_empty() || self.text.len() > 64 * 1024 {
            return Err(SearchError::Pattern("Enter a pattern up to 64 KiB".into()));
        }
        RegexBuilder::new(&if self.regex {
            self.text.clone()
        } else {
            regex::escape(&self.text)
        })
        .case_insensitive(!self.case_sensitive)
        .multi_line(true)
        .size_limit(10 * 1024 * 1024)
        .dfa_size_limit(16 * 1024 * 1024)
        .build()
        .map_err(|e| SearchError::Pattern(e.to_string()))
    }
}
#[derive(Debug, Clone)]
pub struct Hit {
    pub path: PathBuf,
    pub bytes: std::ops::Range<usize>,
    pub line: usize,
    pub column: usize,
    pub preview: String,
}
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub files: usize,
    pub matches: usize,
    pub skipped: usize,
    pub truncated: bool,
}
#[derive(Debug)]
pub enum Event {
    Hit(Hit),
    Done(Stats),
    Failed(SearchError),
}
pub enum Scope {
    Workspace(PathBuf),
    Document(TextSnapshot),
}
pub struct Job {
    pub events: mpsc::Receiver<Event>,
    cancel: Arc<AtomicBool>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
fn whole(text: &str, range: std::ops::Range<usize>, required: bool) -> bool {
    !required
        || (!text[..range.start].chars().next_back().is_some_and(word)
            && !text[range.end..].chars().next().is_some_and(word))
}
fn text(path: &Path) -> Result<String, SearchError> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > FILE_LIMIT {
        return Err(SearchError::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > FILE_LIMIT {
        return Err(SearchError::TooLarge);
    }
    String::from_utf8(bytes)
        .map_err(|_| SearchError::Pattern("Binary or non-UTF-8 file skipped".into()))
}
fn confined(root: &Path, path: &Path) -> Result<PathBuf, SearchError> {
    if !path.starts_with(root) {
        return Err(SearchError::InvalidScope);
    }
    let mut cursor = root.to_owned();
    for c in path
        .strip_prefix(root)
        .map_err(|_| SearchError::InvalidScope)?
        .components()
    {
        if !matches!(c, std::path::Component::Normal(_)) {
            return Err(SearchError::InvalidScope);
        }
        cursor.push(c);
        if std::fs::symlink_metadata(&cursor)?.file_type().is_symlink() {
            return Err(SearchError::InvalidScope);
        }
    }
    let p = std::fs::canonicalize(path)?;
    if !p.starts_with(root) {
        return Err(SearchError::InvalidScope);
    }
    Ok(p)
}
fn scan_text(
    query: &Query,
    regex: &Regex,
    path: &Path,
    source: &str,
    stats: &mut Stats,
    cancel: &AtomicBool,
    emit: &impl Fn(Hit) -> bool,
) -> bool {
    let indexed = Buffer::from_text(source);
    stats.files += 1;
    for found in regex.find_iter(source) {
        if cancel.load(Ordering::Relaxed) {
            return false;
        }
        if !whole(source, found.range(), query.whole_word) {
            continue;
        }
        if stats.matches >= RESULT_LIMIT {
            stats.truncated = true;
            return false;
        }
        let Ok(offset) = indexed.byte_to_char(found.start()) else {
            continue;
        };
        let Ok((row, col)) = indexed.position(offset) else {
            continue;
        };
        let preview = indexed
            .line(row)
            .unwrap_or_default()
            .trim_end_matches(['\r', '\n'])
            .chars()
            .take(180)
            .collect();
        stats.matches += 1;
        if !emit(Hit {
            path: path.into(),
            bytes: found.range(),
            line: row + 1,
            column: col + 1,
            preview,
        }) {
            return false;
        }
    }
    true
}
impl Job {
    pub fn start(scope: Scope, query: Query) -> Self {
        let (tx, events) = mpsc::sync_channel(64);
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let regex = query.compile()?;
                let mut stats = Stats::default();
                let emit = |hit| tx.send(Event::Hit(hit)).is_ok();
                match scope {
                    Scope::Document(snapshot) => {
                        if snapshot.rope.len_bytes() as u64 > FILE_LIMIT {
                            return Err(SearchError::TooLarge);
                        }
                        scan_text(
                            &query,
                            &regex,
                            &snapshot.path,
                            &snapshot.rope.to_string(),
                            &mut stats,
                            &flag,
                            &emit,
                        );
                    }
                    Scope::Workspace(root) => {
                        let root = std::fs::canonicalize(root)?;
                        let mut overrides = ignore::overrides::OverrideBuilder::new(&root);
                        for glob in &query.include {
                            overrides
                                .add(glob)
                                .map_err(|e| SearchError::Pattern(e.to_string()))?;
                        }
                        for glob in &query.exclude {
                            overrides
                                .add(&format!("!{glob}"))
                                .map_err(|e| SearchError::Pattern(e.to_string()))?;
                        }
                        let mut walk = ignore::WalkBuilder::new(&root);
                        walk.hidden(false)
                            .follow_links(false)
                            .require_git(false)
                            .overrides(
                                overrides
                                    .build()
                                    .map_err(|e| SearchError::Pattern(e.to_string()))?,
                            )
                            .filter_entry(|e| {
                                !matches!(
                                    e.file_name().to_str(),
                                    Some(".git" | "target" | "node_modules" | ".venv" | ".aws")
                                ) && !e.file_name().to_string_lossy().starts_with(".env")
                                    && !e.file_name().to_string_lossy().starts_with(".buture-save-")
                            });
                        for (index, entry) in walk.build().enumerate() {
                            if flag.load(Ordering::Relaxed) {
                                break;
                            }
                            if index >= 100_000 {
                                stats.truncated = true;
                                break;
                            }
                            let entry = match entry {
                                Ok(e) => e,
                                Err(_) => {
                                    stats.skipped += 1;
                                    continue;
                                }
                            };
                            if !entry.file_type().is_some_and(|t| t.is_file()) {
                                continue;
                            }
                            let path = match confined(&root, entry.path()) {
                                Ok(p) => p,
                                Err(_) => {
                                    stats.skipped += 1;
                                    continue;
                                }
                            };
                            let source = match text(&path) {
                                Ok(t) => t,
                                Err(_) => {
                                    stats.skipped += 1;
                                    continue;
                                }
                            };
                            if !scan_text(&query, &regex, &path, &source, &mut stats, &flag, &emit)
                            {
                                break;
                            }
                        }
                    }
                }
                Ok(stats)
            })();
            let _ = tx.send(match result {
                Ok(stats) => Event::Done(stats),
                Err(e) => Event::Failed(e),
            });
        });
        Self { events, cancel }
    }
}
pub fn replacement_edits(
    buffer: &Buffer,
    query: &Query,
    replacement: &str,
) -> Result<Vec<Edit>, SearchError> {
    let regex = query.compile()?;
    let source = buffer.rope().to_string();
    let mut edits = Vec::new();
    for captures in regex.captures_iter(&source) {
        let Some(found) = captures.get(0) else {
            continue;
        };
        if !whole(&source, found.range(), query.whole_word) {
            continue;
        }
        if edits.len() >= RESULT_LIMIT {
            return Err(SearchError::TooLarge);
        }
        let mut text = String::new();
        if query.regex {
            captures.expand(replacement, &mut text);
        } else {
            text.push_str(replacement);
        }
        edits.push(Edit {
            range: buffer.byte_to_char(found.start())?..buffer.byte_to_char(found.end())?,
            text,
        });
    }
    Ok(edits)
}
#[derive(Debug, Clone)]
pub struct Change {
    pub path: PathBuf,
    pub matches: usize,
    pub before: String,
    pub after: String,
}
pub struct Plan {
    documents: Vec<Buffer>,
    pub changes: Vec<Change>,
}
pub struct Confirmed(Plan);
#[derive(Debug)]
pub struct Applied {
    pub files: Vec<PathBuf>,
    pub failure: Option<SearchError>,
}
impl Plan {
    pub fn prepare(
        root: &Path,
        paths: &[PathBuf],
        query: &Query,
        replacement: &str,
        dirty: &[PathBuf],
    ) -> Result<Self, SearchError> {
        let root = std::fs::canonicalize(root)?;
        let paths: BTreeSet<_> = paths.iter().collect();
        let mut total = 0;
        let mut documents = Vec::new();
        let mut changes = Vec::new();
        for path in paths {
            if dirty.contains(path) {
                return Err(SearchError::ModifiedDocument);
            }
            let path = confined(&root, path)?;
            let mut buffer = Buffer::open(&path)?;
            total += buffer.len_bytes();
            if total > 64 * 1024 * 1024 {
                return Err(SearchError::TooLarge);
            }
            let edits = replacement_edits(&buffer, query, replacement)?;
            if edits.is_empty() {
                continue;
            }
            let first = edits[0].range.start;
            let row = buffer.char_to_line(first)?;
            let before = buffer.line(row)?.chars().take(180).collect();
            let count = edits.len();
            buffer.apply_edits(&edits)?;
            let after = buffer
                .line(row.min(buffer.line_count() - 1))?
                .chars()
                .take(180)
                .collect();
            changes.push(Change {
                path,
                matches: count,
                before,
                after,
            });
            documents.push(buffer);
        }
        Ok(Self { documents, changes })
    }
    pub fn confirm(self) -> Confirmed {
        Confirmed(self)
    }
}
impl Confirmed {
    pub fn apply(self) -> Applied {
        let mut files = Vec::new();
        for document in &self.0.documents {
            if let Err(error) = document.check_disk() {
                return Applied {
                    files,
                    failure: Some(error.into()),
                };
            }
        }
        for document in self.0.documents {
            match document.save() {
                Ok(saved) => files.push(saved.path),
                Err(error) => {
                    return Applied {
                        files,
                        failure: Some(error.into()),
                    };
                }
            }
        }
        Applied {
            files,
            failure: None,
        }
    }
}
pub mod index;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignore_unicode_regex_and_replacements_are_live() {
        let root = std::env::temp_dir().join(format!("buture-search-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture");
        std::fs::write(root.join(".gitignore"), "ignored.txt\n").expect("ignore");
        std::fs::write(root.join("a.txt"), "🦀 one One lone\n").expect("text");
        std::fs::write(root.join("ignored.txt"), "one").expect("ignored");
        std::fs::write(root.join(".env"), "one").expect("private");
        let query = Query {
            text: "one".into(),
            whole_word: true,
            ..Default::default()
        };
        let job = Job::start(Scope::Workspace(root.clone()), query.clone());
        let mut hits = Vec::new();
        loop {
            match job
                .events
                .recv_timeout(std::time::Duration::from_secs(3))
                .expect("event")
            {
                Event::Hit(hit) => hits.push(hit),
                Event::Done(stats) => {
                    assert_eq!(stats.matches, 2);
                    break;
                }
                Event::Failed(e) => panic!("{e}"),
            }
        }
        assert_eq!(hits[0].column, 3);
        assert!(hits.iter().all(|h| h.path.ends_with("a.txt")));
        let plan = Plan::prepare(&root, &[root.join("a.txt")], &query, "two", &[]).expect("plan");
        assert_eq!(
            std::fs::read_to_string(root.join("a.txt")).expect("unchanged"),
            "🦀 one One lone\n"
        );
        let applied = plan.confirm().apply();
        assert!(applied.failure.is_none());
        assert_eq!(
            std::fs::read_to_string(root.join("a.txt")).expect("replaced"),
            "🦀 two two lone\n"
        );
        let q = Query {
            text: "(two)".into(),
            regex: true,
            ..Default::default()
        };
        let plan = Plan::prepare(&root, &[root.join("a.txt")], &q, "$1!", &[]).expect("regex plan");
        std::fs::write(root.join("a.txt"), "owner changes").expect("external");
        assert!(plan.confirm().apply().failure.is_some());
        assert_eq!(
            std::fs::read_to_string(root.join("a.txt")).expect("preserved"),
            "owner changes"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
