//! Scalable UTF-8 buffer with structural snapshots, typed errors and conflict-aware saves.
use ropey::Rope;
use std::{
    collections::VecDeque,
    fs,
    io::{self, Read},
    ops::Range,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
const MAX_BYTES: u64 = 128 * 1024 * 1024;
const HISTORY: usize = 256;
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
#[derive(Debug)]
pub enum EditorError {
    Io(io::Error),
    InvalidOffset,
    UnsupportedEncoding,
    TooLarge,
    OutsideWorkspace,
    Conflict(PathBuf),
    NoPath,
}
impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::InvalidOffset => write!(f, "Offset is outside the document or splits UTF-8"),
            Self::UnsupportedEncoding => write!(f, "Only UTF-8 text files are supported"),
            Self::TooLarge => write!(f, "File exceeds the 128 MiB editor limit"),
            Self::OutsideWorkspace => {
                write!(f, "File escapes the workspace or traverses a symlink")
            }
            Self::Conflict(p) => write!(
                f,
                "File changed on disk: {}. Reload or review before saving.",
                p.display()
            ),
            Self::NoPath => write!(f, "Choose a path before saving"),
        }
    }
}
impl std::error::Error for EditorError {}
impl From<io::Error> for EditorError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}
impl LineEnding {
    pub fn text(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}
#[derive(Debug, Clone)]
struct Snapshot {
    rope: Rope,
    id: u64,
}
#[derive(Debug, Clone)]
pub struct Buffer {
    rope: Rope,
    saved: Rope,
    id: u64,
    saved_id: u64,
    pub revision: u64,
    pub path: Option<PathBuf>,
    pub line_ending: LineEnding,
    undo: VecDeque<Snapshot>,
    redo: VecDeque<Snapshot>,
    pub width_hint: usize,
}
impl Buffer {
    pub fn from_text(text: &str) -> Self {
        let rope = Rope::from_str(text);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let line_ending = if text
            .find('\n')
            .is_some_and(|i| i > 0 && text.as_bytes()[i - 1] == b'\r')
        {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        };
        let width_hint = rope.lines().map(|l| l.len_chars()).max().unwrap_or(0);
        Self {
            saved: rope.clone(),
            rope,
            id,
            saved_id: id,
            revision: 0,
            path: None,
            line_ending,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            width_hint,
        }
    }
    pub fn open(path: &Path) -> Result<Self, EditorError> {
        let path = fs::canonicalize(path)?;
        let text = read_utf8(&path)?;
        let mut buffer = Self::from_text(&text);
        buffer.path = Some(path);
        Ok(buffer)
    }
    pub fn rope(&self) -> &Rope {
        &self.rope
    }
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }
    pub fn dirty(&self) -> bool {
        self.id != self.saved_id
    }
    pub fn content_id(&self) -> u64 {
        self.id
    }
    pub fn line(&self, row: usize) -> Result<String, EditorError> {
        self.rope
            .get_line(row)
            .map(|l| l.to_string())
            .ok_or(EditorError::InvalidOffset)
    }
    pub fn line_start(&self, row: usize) -> Result<usize, EditorError> {
        self.rope
            .try_line_to_char(row)
            .map_err(|_| EditorError::InvalidOffset)
    }
    pub fn char_to_line(&self, offset: usize) -> Result<usize, EditorError> {
        self.rope
            .try_char_to_line(offset)
            .map_err(|_| EditorError::InvalidOffset)
    }
    pub fn char_to_byte(&self, offset: usize) -> Result<usize, EditorError> {
        self.rope
            .try_char_to_byte(offset)
            .map_err(|_| EditorError::InvalidOffset)
    }
    pub fn byte_to_char(&self, offset: usize) -> Result<usize, EditorError> {
        let position = self
            .rope
            .try_byte_to_char(offset)
            .map_err(|_| EditorError::InvalidOffset)?;
        if self.char_to_byte(position)? != offset {
            return Err(EditorError::InvalidOffset);
        }
        Ok(position)
    }
    pub fn position(&self, offset: usize) -> Result<(usize, usize), EditorError> {
        let row = self.char_to_line(offset)?;
        Ok((row, offset - self.line_start(row)?))
    }
    pub fn offset(&self, row: usize, column: usize) -> Result<usize, EditorError> {
        let start = self.line_start(row)?;
        let line = self.line(row)?;
        let length = line.trim_end_matches(['\r', '\n']).chars().count();
        if column > length {
            return Err(EditorError::InvalidOffset);
        }
        Ok(start + column)
    }
    pub fn slice(&self, range: Range<usize>) -> Result<String, EditorError> {
        self.rope
            .get_slice(range)
            .map(|s| s.to_string())
            .ok_or(EditorError::InvalidOffset)
    }
    fn remember(&mut self) {
        self.undo.push_back(Snapshot {
            rope: self.rope.clone(),
            id: self.id,
        });
        if self.undo.len() > HISTORY {
            self.undo.pop_front();
        }
        self.redo.clear();
    }
    fn changed(&mut self) {
        self.id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn replace(&mut self, range: Range<usize>, text: &str) -> Result<(), EditorError> {
        if range.start > range.end || range.end > self.len_chars() {
            return Err(EditorError::InvalidOffset);
        }
        if self
            .rope
            .get_slice(range.clone())
            .is_some_and(|s| s == text)
        {
            return Ok(());
        }
        self.remember();
        self.rope.remove(range.clone());
        self.rope.insert(range.start, text);
        self.width_hint = self.width_hint.max(
            text.lines().map(|l| l.chars().count()).max().unwrap_or(0)
                + self.position(range.start)?.1,
        );
        self.changed();
        Ok(())
    }
    pub fn insert(&mut self, offset: usize, text: &str) -> Result<(), EditorError> {
        self.replace(offset..offset, text)
    }
    pub fn delete(&mut self, range: Range<usize>) -> Result<(), EditorError> {
        self.replace(range, "")
    }
    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo.pop_back() {
            self.redo.push_back(Snapshot {
                rope: self.rope.clone(),
                id: self.id,
            });
            self.rope = snapshot.rope;
            self.id = snapshot.id;
            self.revision = self.revision.wrapping_add(1);
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.redo.pop_back() {
            self.undo.push_back(Snapshot {
                rope: self.rope.clone(),
                id: self.id,
            });
            self.rope = snapshot.rope;
            self.id = snapshot.id;
            self.revision = self.revision.wrapping_add(1);
            true
        } else {
            false
        }
    }
    pub fn revert_to_saved(&mut self) {
        self.remember();
        self.rope = self.saved.clone();
        self.id = self.saved_id;
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn save(&self) -> Result<Saved, EditorError> {
        let path = self.path.as_ref().ok_or(EditorError::NoPath)?;
        if fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(EditorError::OutsideWorkspace);
        }
        let current = read_utf8(path)?;
        if Rope::from_str(&current) != self.saved {
            return Err(EditorError::Conflict(path.clone()));
        }
        let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let temporary = path.with_file_name(format!(
            ".buture-save-{}-{sequence}.tmp",
            std::process::id()
        ));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            fs::set_permissions(&temporary, fs::metadata(path)?.permissions())?;
            self.rope.write_to(&mut file)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)?;
            Ok(Saved {
                path: path.clone(),
                id: self.id,
                rope: self.rope.clone(),
            })
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
    pub fn accept_saved(&mut self, saved: Saved) {
        if self.path.as_ref() == Some(&saved.path) {
            self.saved_id = saved.id;
            self.saved = saved.rope;
        }
    }
}
fn read_utf8(path: &Path) -> Result<String, EditorError> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > MAX_BYTES {
        return Err(EditorError::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(EditorError::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| EditorError::UnsupportedEncoding)
}
#[derive(Debug)]
pub struct Saved {
    pub path: PathBuf,
    id: u64,
    rope: Rope,
}
#[derive(Debug)]
pub enum FileEvent {
    Opened(Buffer),
    Saved(Saved),
    Failed { path: PathBuf, error: EditorError },
}
pub struct Files {
    pub events: std::sync::mpsc::Receiver<FileEvent>,
    sender: std::sync::mpsc::Sender<FileEvent>,
}
impl Default for Files {
    fn default() -> Self {
        let (sender, events) = std::sync::mpsc::channel();
        Self { sender, events }
    }
}
impl Files {
    pub fn open(&self, root: PathBuf, path: PathBuf) {
        let tx = self.sender.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let root = fs::canonicalize(root)?;
                if !path.starts_with(&root) {
                    return Err(EditorError::OutsideWorkspace);
                }
                let mut cursor = root.clone();
                for component in path
                    .strip_prefix(&root)
                    .map_err(|_| EditorError::OutsideWorkspace)?
                    .components()
                {
                    if !matches!(component, std::path::Component::Normal(_)) {
                        return Err(EditorError::OutsideWorkspace);
                    }
                    cursor.push(component);
                    if fs::symlink_metadata(&cursor)?.file_type().is_symlink() {
                        return Err(EditorError::OutsideWorkspace);
                    }
                }
                let path = fs::canonicalize(&path)?;
                if !path.starts_with(&root) {
                    return Err(EditorError::OutsideWorkspace);
                }
                Buffer::open(&path)
            })();
            let _ = tx.send(match result {
                Ok(buffer) => FileEvent::Opened(buffer),
                Err(error) => FileEvent::Failed { path, error },
            });
        });
    }
    pub fn save(&self, buffer: Buffer) {
        let tx = self.sender.clone();
        std::thread::spawn(move || {
            let path = buffer.path.clone().unwrap_or_default();
            let event = match buffer.save() {
                Ok(saved) => FileEvent::Saved(saved),
                Err(error) => FileEvent::Failed { path, error },
            };
            let _ = tx.send(event);
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_offsets_undo_and_eol_are_consistent() {
        let mut b = Buffer::from_text("á🦀\r\nnext\r\n");
        assert_eq!(b.line_ending, LineEnding::CrLf);
        assert_eq!(b.offset(1, 2).expect("offset"), 6);
        assert_eq!(b.char_to_byte(2).expect("byte"), 6);
        assert!(b.byte_to_char(1).is_err());
        b.replace(0..2, "Olá").expect("replace");
        assert!(b.dirty());
        assert!(b.undo());
        assert!(!b.dirty());
        assert!(b.redo());
        assert_eq!(b.line(0).expect("line"), "Olá\r\n");
    }
    #[test]
    fn save_does_not_overwrite_external_edits() {
        let path = std::env::temp_dir().join(format!("buture-buffer-{}.txt", std::process::id()));
        fs::write(&path, "original\n").expect("fixture");
        let mut b = Buffer::open(&path).expect("open");
        b.insert(0, "new ").expect("edit");
        let saved = b.save().expect("save");
        b.accept_saved(saved);
        assert!(!b.dirty());
        fs::write(&path, "external").expect("external");
        b.insert(0, "other").expect("edit");
        assert!(matches!(b.save(), Err(EditorError::Conflict(_))));
        assert_eq!(fs::read_to_string(&path).expect("disk"), "external");
        fs::remove_file(path).expect("cleanup");
    }
}
