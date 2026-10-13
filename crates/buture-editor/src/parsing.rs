//! Incremental grammar parsing and original token classification on a bounded background worker.
use ropey::Rope;
use std::{
    collections::BTreeMap,
    ops::Range,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
use tree_sitter::{InputEdit, Language, Node, ParseOptions, Parser, Point, Tree};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LanguageKind {
    Rust,
    C,
    Cpp,
    Python,
    Java,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
    Json,
    Toml,
    Yaml,
    Markdown,
}
impl LanguageKind {
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        Some(match path.extension()?.to_str()? {
            "rs" => Self::Rust,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" => Self::Cpp,
            "py" => Self::Python,
            "java" => Self::Java,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "go" => Self::Go,
            "json" => Self::Json,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "md" | "markdown" => Self::Markdown,
            _ => return None,
        })
    }
    pub fn language(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
            Self::Yaml => tree_sitter_yaml::LANGUAGE.into(),
            Self::Markdown => tree_sitter_md::LANGUAGE.into(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Macro,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub bytes: Range<usize>,
    pub kind: Kind,
}
#[derive(Debug, Clone)]
pub struct Parsed {
    pub revision: u64,
    pub content_id: u64,
    pub language: LanguageKind,
    pub spans: Vec<Span>,
    pub folds: Vec<Range<usize>>,
    pub has_error: bool,
}
#[derive(Debug)]
pub enum ParseError {
    IncompatibleLanguage,
    Cancelled,
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::IncompatibleLanguage => "Parser ABI is incompatible",
                Self::Cancelled => "Parsing exceeded its time limit",
            }
        )
    }
}
impl std::error::Error for ParseError {}
struct State {
    parser: Parser,
    tree: Option<Tree>,
    source: Rope,
    language: LanguageKind,
}
fn point(rope: &Rope, byte: usize) -> Point {
    let row = rope.byte_to_line(byte);
    Point {
        row,
        column: byte - rope.line_to_byte(row),
    }
}
fn boundary(rope: &Rope, byte: usize) -> bool {
    rope.char_to_byte(rope.byte_to_char(byte)) == byte
}
fn edit(old: &Rope, new: &Rope) -> InputEdit {
    let prefix = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    let start = old
        .char_to_byte(old.byte_to_char(prefix))
        .min(new.char_to_byte(new.byte_to_char(prefix)));
    let suffix = old
        .bytes_at(old.len_bytes())
        .reversed()
        .zip(new.bytes_at(new.len_bytes()).reversed())
        .take(old.len_bytes().min(new.len_bytes()) - start)
        .take_while(|(a, b)| a == b)
        .count();
    let mut old_end = old.len_bytes() - suffix;
    let mut new_end = new.len_bytes() - suffix;
    while !boundary(old, old_end) || !boundary(new, new_end) {
        old_end += 1;
        new_end += 1;
    }
    InputEdit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position: point(old, start),
        old_end_position: point(old, old_end),
        new_end_position: point(new, new_end),
    }
}
fn classify(node: Node<'_>) -> Option<Kind> {
    let kind = node.kind();
    if kind.contains("comment") {
        return Some(Kind::Comment);
    }
    if matches!(
        kind,
        "string"
            | "string_literal"
            | "raw_string_literal"
            | "interpreted_string_literal"
            | "char_literal"
            | "character_literal"
            | "quoted_scalar"
            | "string_scalar"
    ) {
        return Some(Kind::String);
    }
    if matches!(
        kind,
        "integer"
            | "float"
            | "number"
            | "integer_literal"
            | "float_literal"
            | "decimal_integer_literal"
            | "hex_integer_literal"
            | "int_literal"
            | "integer_scalar"
            | "float_scalar"
    ) {
        return Some(Kind::Number);
    }
    if matches!(
        kind,
        "type_identifier" | "primitive_type" | "predefined_type"
    ) {
        return Some(Kind::Type);
    }
    if node.child_count() == 0
        && [
            "fn",
            "let",
            "mut",
            "pub",
            "use",
            "mod",
            "struct",
            "enum",
            "impl",
            "trait",
            "match",
            "if",
            "else",
            "return",
            "for",
            "while",
            "loop",
            "in",
            "async",
            "await",
            "const",
            "static",
            "class",
            "def",
            "import",
            "from",
            "try",
            "catch",
            "function",
            "var",
            "new",
            "interface",
            "type",
            "export",
            "package",
            "func",
            "true",
            "false",
            "null",
            "None",
            "self",
            "Self",
            "public",
            "private",
            "protected",
            "void",
            "int",
            "double",
            "float",
            "char",
            "bool",
            "using",
            "namespace",
            "template",
            "switch",
            "case",
            "break",
            "continue",
            "extends",
            "implements",
            "throw",
            "throws",
            "yield",
            "with",
            "as",
            "where",
        ]
        .contains(&kind)
    {
        return Some(Kind::Keyword);
    }
    if matches!(
        kind,
        "identifier" | "field_identifier" | "property_identifier"
    ) && let Some(parent) = node.parent()
    {
        let p = parent.kind();
        if p.contains("macro") {
            return Some(Kind::Macro);
        }
        if (p.contains("function") || p.contains("method"))
            && parent.child_by_field_name("name") == Some(node)
        {
            return Some(Kind::Function);
        }
        if matches!(p, "call_expression" | "function_declarator")
            && parent
                .child_by_field_name("function")
                .or_else(|| parent.child_by_field_name("declarator"))
                == Some(node)
        {
            return Some(Kind::Function);
        }
    }
    None
}
impl State {
    fn new(language: LanguageKind) -> Result<Self, ParseError> {
        let mut parser = Parser::new();
        parser
            .set_language(&language.language())
            .map_err(|_| ParseError::IncompatibleLanguage)?;
        Ok(Self {
            parser,
            tree: None,
            source: Rope::new(),
            language,
        })
    }
    fn parse(
        &mut self,
        source: Rope,
        revision: u64,
        content_id: u64,
    ) -> Result<Parsed, ParseError> {
        if let Some(tree) = &mut self.tree {
            tree.edit(&edit(&self.source, &source));
        }
        let began = Instant::now();
        let mut progress = |_: &tree_sitter::ParseState| began.elapsed() > Duration::from_secs(2);
        let parsed_tree = self.parser.parse_with_options(
            &mut |byte, _| {
                if byte >= source.len_bytes() {
                    return &[] as &[u8];
                }
                let (chunk, start, _, _) = source.chunk_at_byte(byte);
                &chunk.as_bytes()[byte - start..]
            },
            self.tree.as_ref(),
            Some(ParseOptions::new().progress_callback(&mut progress)),
        );
        let Some(tree) = parsed_tree else {
            self.tree = None;
            self.parser.reset();
            self.source = source;
            return Err(ParseError::Cancelled);
        };
        self.source = source.clone();
        self.tree = Some(tree.clone());
        let mut spans = Vec::new();
        let mut folds = Vec::new();
        let mut pending = vec![tree.root_node()];
        while let Some(node) = pending.pop() {
            if began.elapsed() > Duration::from_secs(3) {
                return Err(ParseError::Cancelled);
            }
            let start = node.start_position();
            let end = node.end_position();
            if node.is_named()
                && end.row > start.row + 1
                && matches!(
                    node.kind(),
                    "block"
                        | "class_body"
                        | "declaration_list"
                        | "object"
                        | "array"
                        | "list"
                        | "function_definition"
                        | "function_item"
                        | "dictionary"
                        | "section"
                )
            {
                folds.push(start.row..end.row + 1);
            }
            if let Some(kind) = classify(node) {
                spans.push(Span {
                    bytes: node.byte_range(),
                    kind,
                });
                continue;
            }
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            pending.extend(children.into_iter().rev());
        }
        spans.sort_by_key(|s| s.bytes.start);
        folds.sort_by_key(|r| (r.start, r.end));
        folds.dedup();
        let result = Parsed {
            revision,
            content_id,
            language: self.language,
            spans,
            folds,
            has_error: tree.root_node().has_error(),
        };
        self.source = source;
        self.tree = Some(tree);
        Ok(result)
    }
}
struct Request {
    path: PathBuf,
    source: Rope,
    revision: u64,
    content_id: u64,
    language: LanguageKind,
}
pub struct Event {
    pub path: PathBuf,
    pub result: Result<Parsed, ParseError>,
}
pub struct Service {
    pub events: mpsc::Receiver<Event>,
    sender: mpsc::SyncSender<Request>,
}
impl Default for Service {
    fn default() -> Self {
        let (sender, rx) = mpsc::sync_channel::<Request>(16);
        let (tx, events) = mpsc::channel();
        std::thread::spawn(move || {
            let mut states: BTreeMap<PathBuf, State> = BTreeMap::new();
            while let Ok(first) = rx.recv() {
                let mut requests = BTreeMap::from([(first.path.clone(), first)]);
                for request in rx.try_iter() {
                    requests.insert(request.path.clone(), request);
                }
                for (path, request) in requests {
                    let result = (|| {
                        if !states.contains_key(&path) || states[&path].language != request.language
                        {
                            states.insert(path.clone(), State::new(request.language)?);
                        }
                        states
                            .get_mut(&path)
                            .ok_or(ParseError::IncompatibleLanguage)?
                            .parse(request.source, request.revision, request.content_id)
                    })();
                    if tx.send(Event { path, result }).is_err() {
                        return;
                    }
                }
            }
        });
        Self { events, sender }
    }
}
impl Service {
    pub fn request(&self, path: PathBuf, source: Rope, revision: u64, content_id: u64) -> bool {
        let Some(language) = LanguageKind::from_path(&path) else {
            return false;
        };
        self.sender
            .try_send(Request {
                path,
                source,
                revision,
                content_id,
                language,
            })
            .is_ok()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_grammars_load_and_parse() {
        for (language, source) in [
            (LanguageKind::Rust, "fn main() { let n = 1; }"),
            (LanguageKind::C, "int main(void) { return 0; }"),
            (LanguageKind::Cpp, "class X {}; int main() {}"),
            (LanguageKind::Python, "def main():\n    return 1\n"),
            (
                LanguageKind::Java,
                "class A { public static void main(String[] args) {} }",
            ),
            (LanguageKind::JavaScript, "function f(){ return 1; }"),
            (LanguageKind::TypeScript, "const x: number = 1;"),
            (LanguageKind::Tsx, "const x = <div />;"),
            (LanguageKind::Go, "package main\nfunc main() {}\n"),
            (LanguageKind::Json, "{\"x\":1}"),
            (LanguageKind::Toml, "[package]\nname='x'\n"),
            (LanguageKind::Yaml, "key: value\n"),
            (LanguageKind::Markdown, "# Title\n\nText **strong**.\n"),
        ] {
            let parsed = State::new(language)
                .expect("ABI")
                .parse(Rope::from_str(source), 0, 1)
                .expect("parse");
            assert!(!parsed.has_error, "{language:?}");
        }
    }
    #[test]
    fn incremental_utf8_edits_match_fresh_parse() {
        let mut state = State::new(LanguageKind::Rust).expect("parser");
        state
            .parse(Rope::from_str("fn main(){ let x = \"é\"; }"), 0, 1)
            .expect("initial");
        let source = Rope::from_str("fn main(){ let x = \"🦀\"; println!(\"{x}\"); }");
        let incremental = state.parse(source.clone(), 1, 2).expect("incremental");
        let mut fresh = State::new(LanguageKind::Rust).expect("fresh");
        let full = fresh.parse(source, 1, 2).expect("full");
        assert_eq!(incremental.spans, full.spans);
        assert_eq!(
            state.tree.expect("tree").root_node().to_sexp(),
            fresh.tree.expect("tree").root_node().to_sexp()
        );
    }
}
