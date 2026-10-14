//! Language-independent LSP lifecycle over a bounded local subprocess transport.
use buture_editor::TextSnapshot;
use buture_protocol::{Process, ProtocolError, Spec, Value};
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
#[derive(Debug)]
pub enum LspError {
    Transport(ProtocolError),
    NotReady,
    DocumentNotOpen,
    DocumentTooLarge,
    Timeout(String),
    Server(String),
    UnsupportedEncoding,
}
impl std::fmt::Display for LspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "{e}"),
            Self::NotReady => write!(f, "Language server is not initialized"),
            Self::DocumentNotOpen => write!(f, "Document is not synchronized with this server"),
            Self::DocumentTooLarge => write!(
                f,
                "Language service limit: 64 documents, 2 MiB per document"
            ),
            Self::Timeout(method) => write!(f, "Language request timed out: {method}"),
            Self::Server(message) => write!(f, "Language server error: {message}"),
            Self::UnsupportedEncoding => write!(f, "Server did not negotiate UTF-16 positions"),
        }
    }
}
impl std::error::Error for LspError {}
impl From<ProtocolError> for LspError {
    fn from(e: ProtocolError) -> Self {
        Self::Transport(e)
    }
}
#[derive(Clone)]
pub struct Server {
    pub process: Spec,
    pub language: String,
    pub initialization_options: Value,
}
pub struct ConfirmedServer(Server);
impl Server {
    pub fn confirm(self) -> ConfirmedServer {
        ConfirmedServer(self)
    }
}
pub enum Event {
    Ready(Value),
    Response {
        method: String,
        result: Value,
        context: Option<TextSnapshot>,
    },
    Notification {
        method: String,
        params: Value,
    },
    Warning(LspError),
    Stopped,
}
enum Request {
    Sync {
        documents: Vec<TextSnapshot>,
        saved: Vec<TextSnapshot>,
    },
    Feature {
        method: String,
        params: Value,
        context: Option<TextSnapshot>,
    },
}
pub struct Client {
    pub events: mpsc::Receiver<Event>,
    sender: mpsc::SyncSender<Request>,
    stop: Arc<AtomicBool>,
}
impl Client {
    pub fn sync(
        &self,
        documents: Vec<TextSnapshot>,
        saved: Vec<TextSnapshot>,
    ) -> Result<(), LspError> {
        self.sender
            .try_send(Request::Sync { documents, saved })
            .map_err(|_| LspError::Transport(ProtocolError::QueueUnavailable))
    }
    pub fn request(
        &self,
        method: &str,
        params: Value,
        context: Option<TextSnapshot>,
    ) -> Result<(), LspError> {
        self.sender
            .try_send(Request::Feature {
                method: method.into(),
                params,
                context,
            })
            .map_err(|_| LspError::Transport(ProtocolError::QueueUnavailable))
    }
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.shutdown();
    }
}
impl ConfirmedServer {
    pub fn start(self) -> Client {
        let (sender, rx) = mpsc::sync_channel(64);
        let (tx, events) = mpsc::sync_channel(64);
        let stop = Arc::new(AtomicBool::new(false));
        let cancel = stop.clone();
        std::thread::spawn(move || {
            if let Err(e) = run(self.0, rx, &tx, &cancel) {
                let _ = tx.send(Event::Warning(e));
            }
            let _ = tx.send(Event::Stopped);
        });
        Client {
            events,
            sender,
            stop,
        }
    }
}
pub fn file_uri(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    let mut uri = if raw.starts_with('/') {
        "file://".to_string()
    } else {
        "file:///".to_string()
    };
    for b in raw.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~:".contains(&b) {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}
pub fn uri_path(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://")?;
    let raw = raw.strip_prefix("localhost").unwrap_or(raw);
    if !raw.starts_with('/') {
        return None;
    }
    let mut bytes = Vec::new();
    let mut i = 0;
    let input = raw.as_bytes();
    while i < input.len() {
        if input[i] == b'%' {
            if i + 2 >= input.len() {
                return None;
            }
            let hex = std::str::from_utf8(&input[i + 1..i + 3]).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            bytes.push(input[i]);
            i += 1;
        }
    }
    let mut decoded = String::from_utf8(bytes).ok()?;
    if cfg!(windows) && decoded.as_bytes().get(2) == Some(&b':') {
        decoded.remove(0);
    }
    let path = PathBuf::from(decoded);
    path.is_absolute().then_some(path)
}
pub fn position(snapshot: &TextSnapshot, character: usize) -> Option<Value> {
    if character > snapshot.rope.len_chars() {
        return None;
    }
    let line = snapshot.rope.char_to_line(character);
    let start = snapshot.rope.line_to_char(line);
    let column = snapshot
        .rope
        .slice(start..character)
        .chars()
        .map(char::len_utf16)
        .sum::<usize>();
    Some(json!({"line":line,"character":column}))
}
pub fn character(snapshot: &TextSnapshot, position: &Value) -> Option<usize> {
    let line = position["line"].as_u64()? as usize;
    let column = position["character"].as_u64()? as usize;
    if line >= snapshot.rope.len_lines() {
        return None;
    }
    let start = snapshot.rope.line_to_char(line);
    let mut utf16 = 0;
    for (index, c) in snapshot.rope.line(line).chars().enumerate() {
        if utf16 == column {
            return Some(start + index);
        }
        if ['\r', '\n'].contains(&c) {
            return None;
        }
        utf16 += c.len_utf16();
        if utf16 > column {
            return None;
        }
    }
    (utf16 == column).then_some(start + snapshot.rope.line(line).len_chars())
}
pub fn supports_language(language: &str, path: &Path) -> bool {
    matches!(
        (
            language,
            path.extension().and_then(|e| e.to_str()).unwrap_or("")
        ),
        ("rust", "rs")
            | ("python", "py")
            | ("javascript", "js")
            | ("typescript", "ts" | "tsx")
            | ("go", "go")
            | ("java", "java")
            | ("c", "c" | "h")
            | ("cpp", "cpp" | "hpp" | "cc")
    )
}
struct Document {
    signature: (u64, u64),
    version: i32,
}
struct Pending {
    method: String,
    started: Instant,
    context: Option<TextSnapshot>,
}
fn notify(process: &Process, method: &str, params: Value) -> Result<(), LspError> {
    process.send(json!({"jsonrpc":"2.0","method":method,"params":params}))?;
    Ok(())
}
fn synchronize(
    process: &Process,
    server: &Server,
    docs: &mut BTreeMap<String, Document>,
    documents: Vec<TextSnapshot>,
    saved: Vec<TextSnapshot>,
    include_text: bool,
    tx: &mpsc::SyncSender<Event>,
) -> Result<(), LspError> {
    let incoming: BTreeMap<_, _> = documents
        .into_iter()
        .filter(|s| {
            s.path.starts_with(&server.process.cwd) && supports_language(&server.language, &s.path)
        })
        .map(|s| (file_uri(&s.path), s))
        .collect();
    let closed: Vec<_> = docs
        .keys()
        .filter(|uri| !incoming.contains_key(*uri))
        .cloned()
        .collect();
    for uri in closed {
        notify(
            process,
            "textDocument/didClose",
            json!({"textDocument":{"uri":uri}}),
        )?;
        docs.remove(&uri);
    }
    for (uri, snapshot) in incoming.into_iter().take(64) {
        if snapshot.rope.len_bytes() > 2 * 1024 * 1024 {
            let _ = tx.send(Event::Warning(LspError::DocumentTooLarge));
            continue;
        }
        let signature = (snapshot.revision, snapshot.content_id);
        if let Some(doc) = docs.get_mut(&uri) {
            if doc.signature != signature {
                doc.version = doc.version.checked_add(1).ok_or(LspError::NotReady)?;
                notify(
                    process,
                    "textDocument/didChange",
                    json!({"textDocument":{"uri":uri,"version":doc.version},"contentChanges":[{"text":snapshot.rope.to_string()}]}),
                )?;
                doc.signature = signature;
            }
        } else {
            notify(
                process,
                "textDocument/didOpen",
                json!({"textDocument":{"uri":uri,"languageId":server.language,"version":1,"text":snapshot.rope.to_string()}}),
            )?;
            docs.insert(
                uri,
                Document {
                    signature,
                    version: 1,
                },
            );
        }
    }
    for snapshot in saved {
        let uri = file_uri(&snapshot.path);
        if docs.contains_key(&uri) {
            let mut params = json!({"textDocument":{"uri":uri}});
            if include_text && snapshot.rope.len_bytes() <= 2 * 1024 * 1024 {
                params["text"] = json!(snapshot.rope.to_string());
            }
            notify(process, "textDocument/didSave", params)?;
        }
    }
    Ok(())
}
fn run(
    server: Server,
    rx: mpsc::Receiver<Request>,
    tx: &mpsc::SyncSender<Event>,
    stop: &AtomicBool,
) -> Result<(), LspError> {
    let process = Process::start(server.process.clone());
    let root = file_uri(&server.process.cwd);
    let mut ready = false;
    let mut initialized = false;
    let mut next = 1u64;
    let mut pending = BTreeMap::<u64, Pending>::new();
    let mut documents = BTreeMap::new();
    let mut latest = None;
    let mut include_text = false;
    let mut shutdown = None::<Instant>;
    let mut exiting = None::<Instant>;
    loop {
        if stop.load(Ordering::Relaxed) && shutdown.is_none() {
            if ready {
                process
                    .send(json!({"jsonrpc":"2.0","id":next,"method":"shutdown","params":null}))?;
                pending.insert(
                    next,
                    Pending {
                        method: "shutdown".into(),
                        started: Instant::now(),
                        context: None,
                    },
                );
                next += 1;
            } else {
                process.stop();
            }
            shutdown = Some(Instant::now());
        }
        for event in process.events.try_iter().take(128) {
            match event {
                buture_protocol::Event::Ready => {
                    process.send(json!({"jsonrpc":"2.0","id":next,"method":"initialize","params":{"processId":std::process::id(),"clientInfo":{"name":"Buture Code","version":"0.1.0"},"rootUri":root,"capabilities":{"general":{"positionEncodings":["utf-16"]},"textDocument":{"synchronization":{"didSave":true},"completion":{"completionItem":{"snippetSupport":false}},"hover":{"contentFormat":["plaintext"]}},"workspace":{"workspaceFolders":true,"configuration":true}},"workspaceFolders":[{"uri":root,"name":"Workspace"}],"initializationOptions":server.initialization_options}}))?;
                    pending.insert(
                        next,
                        Pending {
                            method: "initialize".into(),
                            started: Instant::now(),
                            context: None,
                        },
                    );
                    next += 1;
                }
                buture_protocol::Event::Message(value) => {
                    if let Some(id) = value["id"].as_u64()
                        && value.get("method").is_none()
                    {
                        if let Some(request) = pending.remove(&id) {
                            if let Some(error) = value.get("error") {
                                let _ = tx.send(Event::Warning(LspError::Server(
                                    error["message"]
                                        .as_str()
                                        .unwrap_or("request rejected")
                                        .chars()
                                        .take(4096)
                                        .collect(),
                                )));
                                if request.method == "initialize" {
                                    return Err(LspError::NotReady);
                                }
                            } else if request.method == "initialize" {
                                let capabilities = value["result"]["capabilities"].clone();
                                if capabilities["positionEncoding"]
                                    .as_str()
                                    .is_some_and(|s| s != "utf-16")
                                {
                                    return Err(LspError::UnsupportedEncoding);
                                }
                                include_text =
                                    capabilities["textDocumentSync"]["save"]["includeText"]
                                        .as_bool()
                                        .unwrap_or(false);
                                notify(&process, "initialized", json!({}))?;
                                ready = true;
                                initialized = true;
                                let _ = tx.send(Event::Ready(capabilities));
                            } else if request.method == "shutdown" {
                                notify(&process, "exit", Value::Null)?;
                                exiting = Some(Instant::now());
                            } else {
                                let _ = tx.send(Event::Response {
                                    method: request.method,
                                    result: value["result"].clone(),
                                    context: request.context,
                                });
                            }
                        }
                    } else if let Some(method) = value["method"].as_str() {
                        if let Some(id) = value.get("id") {
                            let result = match method {
                                "workspace/configuration" => json!(
                                    value["params"]["items"]
                                        .as_array()
                                        .map(|items| items
                                            .iter()
                                            .map(|_| Value::Null)
                                            .collect::<Vec<_>>())
                                        .unwrap_or_default()
                                ),
                                "workspace/workspaceFolders" => {
                                    json!([{"uri":root,"name":"Workspace"}])
                                }
                                "window/showMessageRequest" => Value::Null,
                                "workspace/applyEdit" => {
                                    json!({"applied":false,"failureReason":"Interactive review required"})
                                }
                                _ => {
                                    process.send(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Unsupported client request"}}))?;
                                    continue;
                                }
                            };
                            process.send(json!({"jsonrpc":"2.0","id":id,"result":result}))?;
                        } else {
                            let _ = tx.send(Event::Notification {
                                method: method.into(),
                                params: value["params"].clone(),
                            });
                        }
                    }
                }
                buture_protocol::Event::Failed(e) => return Err(e.into()),
                buture_protocol::Event::Exited(code) => {
                    if shutdown.is_none() {
                        return Err(LspError::Server(format!(
                            "Process exited with status {code:?}"
                        )));
                    }
                    return Ok(());
                }
            }
        }
        if initialized && let Some((all, saved)) = latest.take() {
            synchronize(
                &process,
                &server,
                &mut documents,
                all,
                saved,
                include_text,
                tx,
            )?;
        }
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(Request::Sync {
                documents: all,
                saved,
            }) => {
                if initialized && shutdown.is_none() {
                    synchronize(
                        &process,
                        &server,
                        &mut documents,
                        all,
                        saved,
                        include_text,
                        tx,
                    )?;
                } else {
                    latest = Some((all, saved));
                }
            }
            Ok(Request::Feature {
                method,
                params,
                context,
            }) => {
                if !ready || shutdown.is_some() {
                    let _ = tx.send(Event::Warning(LspError::NotReady));
                    continue;
                }
                if pending.len() >= 128 {
                    return Err(LspError::Transport(ProtocolError::QueueUnavailable));
                }
                if let Some(snapshot) = &context
                    && documents
                        .get(&file_uri(&snapshot.path))
                        .is_none_or(|doc| doc.signature != (snapshot.revision, snapshot.content_id))
                {
                    let _ = tx.send(Event::Warning(LspError::DocumentNotOpen));
                    continue;
                }
                process.send(json!({"jsonrpc":"2.0","id":next,"method":method,"params":params}))?;
                pending.insert(
                    next,
                    Pending {
                        method,
                        started: Instant::now(),
                        context,
                    },
                );
                next += 1;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                process.stop();
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let expired: Vec<_> = pending
            .iter()
            .filter(|(_, p)| {
                p.started.elapsed()
                    > if p.method == "initialize" {
                        Duration::from_secs(45)
                    } else {
                        Duration::from_secs(15)
                    }
            })
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            if let Some(p) = pending.remove(&id) {
                let fatal = p.method == "initialize";
                let _ = tx.send(Event::Warning(LspError::Timeout(p.method)));
                if fatal {
                    return Err(LspError::NotReady);
                }
            }
        }
        if shutdown.is_some_and(|started| started.elapsed() > Duration::from_secs(3))
            || exiting.is_some_and(|started| started.elapsed() > Duration::from_secs(1))
        {
            process.stop();
            return Ok(());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(unix)]
    fn local_mock_server_observes_complete_lifecycle() {
        let executable = PathBuf::from("/usr/bin/python3");
        if !executable.is_file() {
            return;
        }
        let root =
            std::env::temp_dir().join(format!("buture-lsp-lifecycle-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture");
        let log = root.join("events.txt");
        let code = r#"import sys,json
log=sys.argv[1]
while True:
 line=sys.stdin.buffer.readline()
 if not line: break
 n=int(line.split(b':')[1]);sys.stdin.buffer.readline();v=json.loads(sys.stdin.buffer.read(n));m=v.get('method','')
 with open(log,'a') as f: f.write(m+'\n')
 if m=='exit': break
 if 'id' in v:
  r={'capabilities':{'positionEncoding':'utf-16','textDocumentSync':{'openClose':True,'change':1,'save':{'includeText':True}}}} if m=='initialize' else {'observed':m}
  b=json.dumps({'jsonrpc':'2.0','id':v['id'],'result':r}).encode();sys.stdout.buffer.write(b'Content-Length: '+str(len(b)).encode()+b'\r\n\r\n'+b);sys.stdout.buffer.flush()
"#;
        let server = Server {
            process: Spec {
                executable,
                args: vec![
                    "-u".into(),
                    "-c".into(),
                    code.into(),
                    log.to_string_lossy().into_owned(),
                ],
                cwd: root.clone(),
            },
            language: "rust".into(),
            initialization_options: Value::Null,
        };
        let client = server.confirm().start();
        loop {
            match client
                .events
                .recv_timeout(Duration::from_secs(3))
                .expect("initialize")
            {
                Event::Ready(_) => break,
                Event::Warning(e) => panic!("{e}"),
                _ => {}
            }
        }
        let file = root.join("main.rs");
        std::fs::write(&file, "fn main() {}\n").expect("source");
        let mut buffer = buture_editor::Buffer::from_text("fn main() {}\n");
        buffer.path = Some(file);
        client.sync(vec![buffer.snapshot()], vec![]).expect("open");
        buffer.insert(0, "// café 🦀\n").expect("edit");
        let saved = buffer.save().expect("save");
        client
            .sync(vec![buffer.snapshot()], vec![saved.snapshot()])
            .expect("change/save");
        client.sync(vec![], vec![]).expect("close");
        client
            .request("fixture/ping", Value::Null, None)
            .expect("ping");
        loop {
            match client
                .events
                .recv_timeout(Duration::from_secs(3))
                .expect("response")
            {
                Event::Response { method, .. } if method == "fixture/ping" => break,
                Event::Warning(e) => panic!("{e}"),
                _ => {}
            }
        }
        client.shutdown();
        loop {
            if matches!(
                client
                    .events
                    .recv_timeout(Duration::from_secs(4))
                    .expect("shutdown"),
                Event::Stopped
            ) {
                break;
            }
        }
        let methods = std::fs::read_to_string(log).expect("log");
        for method in [
            "initialize",
            "initialized",
            "textDocument/didOpen",
            "textDocument/didChange",
            "textDocument/didSave",
            "textDocument/didClose",
            "shutdown",
            "exit",
        ] {
            assert!(
                methods.lines().any(|m| m == method),
                "missing {method}: {methods}"
            );
        }
        drop(client);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn unicode_positions_roundtrip_and_reject_half_surrogates() {
        let buffer = buture_editor::Buffer::from_text("a🦀é\r\nnext");
        let snapshot = buffer.snapshot();
        assert_eq!(
            position(&snapshot, 2),
            Some(json!({"line":0,"character":3}))
        );
        assert_eq!(character(&snapshot, &json!({"line":0,"character":2})), None);
        for char in [0, 1, 2, 3, 5, 6, 7] {
            let pos = position(&snapshot, char).expect("pos");
            if snapshot.rope.char(char.min(snapshot.rope.len_chars() - 1)) != '\n' {
                assert_eq!(character(&snapshot, &pos), Some(char));
            }
        }
        let path = std::env::temp_dir().join("café space.rs");
        assert_eq!(uri_path(&file_uri(&path)), Some(path));
        assert!(uri_path("https://example.com/a").is_none());
    }
}
