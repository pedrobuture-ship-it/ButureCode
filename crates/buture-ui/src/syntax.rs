//! Buture Emerald lexical fallback, replaced by incremental grammar captures when available.
use crate::{Theme, theme::Color};
use egui::{FontId, TextFormat, text::LayoutJob};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Macro,
}
pub fn color(theme: &Theme, kind: Kind) -> Color {
    match kind {
        Kind::Plain => theme.text.primary,
        Kind::Keyword => theme.syntax.keyword,
        Kind::Function => theme.syntax.function,
        Kind::Type => theme.syntax.types,
        Kind::String => theme.syntax.string,
        Kind::Number => theme.syntax.number,
        Kind::Comment => theme.syntax.comment,
        Kind::Macro => theme.syntax.macros,
    }
}
pub fn tokens(text: &str) -> Vec<(std::ops::Range<usize>, Kind)> {
    let chars: Vec<_> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        let (_, c) = chars[i];
        let kind = if (c == '/' && chars.get(i + 1).is_some_and(|(_, c)| *c == '/')) || c == '#' {
            i = chars.len();
            Kind::Comment
        } else if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            i += 1;
            while i < chars.len() {
                let c = chars[i].1;
                i += 1;
                if c == '\\' {
                    i = (i + 1).min(chars.len());
                } else if c == quote {
                    break;
                }
            }
            Kind::String
        } else if c.is_ascii_digit() {
            i += 1;
            while i < chars.len()
                && (chars[i].1.is_ascii_alphanumeric() || ['.', '_'].contains(&chars[i].1))
            {
                i += 1;
            }
            Kind::Number
        } else if c.is_alphabetic() || c == '_' {
            i += 1;
            while i < chars.len() && (chars[i].1.is_alphanumeric() || chars[i].1 == '_') {
                i += 1;
            }
            let end = chars.get(i).map_or(text.len(), |(b, _)| *b);
            let word = &text[chars[start].0..end];
            let next = text[end..].trim_start().chars().next();
            if [
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
            ]
            .contains(&word)
            {
                Kind::Keyword
            } else if next == Some('!') {
                Kind::Macro
            } else if next == Some('(') {
                Kind::Function
            } else if c.is_uppercase() {
                Kind::Type
            } else {
                Kind::Plain
            }
        } else {
            i += 1;
            Kind::Plain
        };
        let end = chars.get(i).map_or(text.len(), |(b, _)| *b);
        out.push((chars[start].0..end, kind));
    }
    out
}
pub fn layout(text: &str, font: FontId, theme: &Theme) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (range, kind) in tokens(text) {
        job.append(
            &text[range],
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: color(theme, kind).into(),
                ..Default::default()
            },
        );
    }
    job
}
pub fn layout_parsed(
    source: &str,
    display: &str,
    map: &[usize],
    byte_start: usize,
    parsed: Option<&buture_editor::parsing::Parsed>,
    font: FontId,
    theme: &Theme,
) -> LayoutJob {
    let Some(parsed) = parsed else {
        return layout(display, font, theme);
    };
    let mut job = LayoutJob::default();
    let mut at = 0;
    let byte_end = byte_start + source.len();
    let display_bytes: Vec<_> = display
        .char_indices()
        .map(|(b, _)| b)
        .chain(std::iter::once(display.len()))
        .collect();
    let first = parsed.spans.partition_point(|s| s.bytes.end <= byte_start);
    for span in parsed.spans[first..]
        .iter()
        .take_while(|s| s.bytes.start < byte_end)
    {
        let start = span.bytes.start.max(byte_start) - byte_start;
        let end = span.bytes.end.min(byte_end) - byte_start;
        let a = source[..start].chars().count();
        let b = source[..end].chars().count();
        let from = map.iter().position(|p| *p == a).unwrap_or(0);
        let to = map.iter().rposition(|p| *p == b).unwrap_or(map.len() - 1);
        let from = display_bytes[from].max(at);
        let to = display_bytes[to].max(from);
        if from > at {
            job.append(
                &display[at..from],
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: theme.text.primary.into(),
                    ..Default::default()
                },
            );
        }
        let kind = match span.kind {
            buture_editor::parsing::Kind::Keyword => Kind::Keyword,
            buture_editor::parsing::Kind::Function => Kind::Function,
            buture_editor::parsing::Kind::Type => Kind::Type,
            buture_editor::parsing::Kind::String => Kind::String,
            buture_editor::parsing::Kind::Number => Kind::Number,
            buture_editor::parsing::Kind::Comment => Kind::Comment,
            buture_editor::parsing::Kind::Macro => Kind::Macro,
        };
        job.append(
            &display[from..to],
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: color(theme, kind).into(),
                ..Default::default()
            },
        );
        at = to;
    }
    if at < display.len() {
        job.append(
            &display[at..],
            0.0,
            TextFormat {
                font_id: font,
                color: theme.text.primary.into(),
                ..Default::default()
            },
        );
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strings_and_comments_do_not_turn_keywords_into_code() {
        let text = "let s = \"fn 123\"; // return";
        let spans = tokens(text);
        assert_eq!(spans[0].1, Kind::Keyword);
        assert!(
            spans
                .iter()
                .any(|(r, k)| *k == Kind::String && &text[r.clone()] == "\"fn 123\"")
        );
        assert_eq!(spans.last().expect("comment").1, Kind::Comment);
    }
}
