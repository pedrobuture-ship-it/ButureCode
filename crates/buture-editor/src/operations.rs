//! Editing transactions are pure buffer operations; no filesystem or process side effects.
use crate::{
    Buffer, Edit, EditorError,
    parsing::{Kind, Parsed},
};
use std::{collections::BTreeSet, ops::Range};
pub fn normalized(ranges: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut ranges = ranges.to_vec();
    ranges.sort_by_key(|r| (r.start, r.end));
    let mut out: Vec<Range<usize>> = Vec::new();
    for r in ranges {
        if let Some(last) = out.last_mut()
            && (r.start < last.end || r.start == last.start)
        {
            last.end = last.end.max(r.end);
        } else {
            out.push(r);
        }
    }
    out
}
pub fn insert(
    buffer: &mut Buffer,
    ranges: &[Range<usize>],
    text: &str,
    auto_close: bool,
) -> Result<Vec<Range<usize>>, EditorError> {
    let ranges = normalized(ranges);
    let mut edits = Vec::new();
    let mut tails = Vec::new();
    let mut skipped = Vec::new();
    let one = text.chars().next().filter(|_| text.chars().count() == 1);
    for range in &ranges {
        if auto_close
            && range.is_empty()
            && one.is_some_and(|c| {
                [')', ']', '}', '"', '\''].contains(&c)
                    && buffer.rope().get_char(range.start) == Some(c)
            })
        {
            edits.push(Edit {
                range: range.clone(),
                text: String::new(),
            });
            tails.push(0);
            skipped.push(true);
            continue;
        }
        let close = if auto_close {
            match one {
                Some('(') => Some(')'),
                Some('[') => Some(']'),
                Some('{') => Some('}'),
                Some('"') => Some('"'),
                Some('\'') => Some('\''),
                _ => None,
            }
        } else {
            None
        };
        let replacement = if let Some(close) = close {
            format!("{text}{}{close}", buffer.slice(range.clone())?)
        } else {
            text.into()
        };
        edits.push(Edit {
            range: range.clone(),
            text: replacement,
        });
        tails.push(usize::from(close.is_some()));
        skipped.push(false);
    }
    let heads = buffer.apply_edits(&edits)?;
    Ok(heads
        .into_iter()
        .zip(tails)
        .zip(skipped)
        .map(|((head, tail), skip)| {
            let p = if skip { head + 1 } else { head - tail };
            p..p
        })
        .collect())
}
pub fn newline(
    buffer: &mut Buffer,
    ranges: &[Range<usize>],
) -> Result<Vec<Range<usize>>, EditorError> {
    let mut edits = Vec::new();
    let mut tails = Vec::new();
    for range in normalized(ranges) {
        let row = buffer.char_to_line(range.start)?;
        let line = buffer.line(row)?;
        let indent: String = line
            .chars()
            .take_while(|c| [' ', '\t'].contains(c))
            .collect();
        let prefix = range
            .start
            .checked_sub(1)
            .and_then(|p| buffer.rope().get_char(p));
        let pair = match prefix {
            Some('{') => Some('}'),
            Some('(') => Some(')'),
            Some('[') => Some(']'),
            _ => None,
        };
        let extra = if pair.is_some() { "    " } else { "" };
        let suffix = if pair.is_some() && buffer.rope().get_char(range.end) == pair {
            format!("{}{indent}", buffer.line_ending.text())
        } else {
            String::new()
        };
        tails.push(suffix.chars().count());
        edits.push(Edit {
            range,
            text: format!("{}{indent}{extra}{suffix}", buffer.line_ending.text()),
        });
    }
    let heads = buffer.apply_edits(&edits)?;
    Ok(heads
        .into_iter()
        .zip(tails)
        .map(|(head, tail)| {
            let p = head - tail;
            p..p
        })
        .collect())
}
pub fn rows(buffer: &Buffer, ranges: &[Range<usize>]) -> Result<BTreeSet<usize>, EditorError> {
    let mut rows = BTreeSet::new();
    for range in normalized(ranges) {
        let first = buffer.char_to_line(range.start)?;
        let last = buffer.char_to_line(if range.end > range.start {
            range.end - 1
        } else {
            range.end
        })?;
        rows.extend(first..=last);
    }
    Ok(rows)
}
fn translated(offset: usize, edits: &[Edit]) -> usize {
    let mut delta = 0i64;
    for edit in edits {
        if edit.range.end <= offset {
            delta += edit.text.chars().count() as i64 - (edit.range.end - edit.range.start) as i64;
        } else if edit.range.start < offset {
            return (edit.range.start as i64 + delta + edit.text.chars().count() as i64).max(0)
                as usize;
        } else {
            break;
        }
    }
    (offset as i64 + delta).max(0) as usize
}
fn apply_preserving(
    buffer: &mut Buffer,
    ranges: &[Range<usize>],
    mut edits: Vec<Edit>,
) -> Result<Vec<Range<usize>>, EditorError> {
    edits.sort_by_key(|e| e.range.start);
    let positions = ranges
        .iter()
        .map(|r| translated(r.start, &edits)..translated(r.end, &edits))
        .collect();
    buffer.apply_edits(&edits)?;
    Ok(positions)
}
pub fn indent(
    buffer: &mut Buffer,
    ranges: &[Range<usize>],
    outdent: bool,
) -> Result<Vec<Range<usize>>, EditorError> {
    let mut edits = Vec::new();
    for row in rows(buffer, ranges)? {
        let start = buffer.line_start(row)?;
        let line = buffer.line(row)?;
        let count = if line.starts_with('\t') {
            1
        } else {
            line.chars().take(4).take_while(|c| *c == ' ').count()
        };
        edits.push(if outdent {
            Edit {
                range: start..start + count,
                text: String::new(),
            }
        } else {
            Edit {
                range: start..start,
                text: "    ".into(),
            }
        });
    }
    apply_preserving(buffer, ranges, edits)
}
pub fn comment(
    buffer: &mut Buffer,
    ranges: &[Range<usize>],
    marker: &str,
) -> Result<Vec<Range<usize>>, EditorError> {
    let rows = rows(buffer, ranges)?;
    let remove = rows.iter().all(|row| {
        buffer
            .line(*row)
            .is_ok_and(|l| l.trim_start().starts_with(marker))
    });
    let mut edits = Vec::new();
    for row in rows {
        let line = buffer.line(row)?;
        let white = line.chars().take_while(|c| [' ', '\t'].contains(c)).count();
        let start = buffer.line_start(row)? + white;
        let count = if remove {
            marker.chars().count()
                + usize::from(
                    line.trim_start()
                        .strip_prefix(marker)
                        .is_some_and(|s| s.starts_with(' ')),
                )
        } else {
            0
        };
        edits.push(Edit {
            range: start..start + count,
            text: if remove {
                String::new()
            } else {
                format!("{marker} ")
            },
        });
    }
    apply_preserving(buffer, ranges, edits)
}
pub fn duplicate_lines(
    buffer: &mut Buffer,
    ranges: &[Range<usize>],
) -> Result<Vec<Range<usize>>, EditorError> {
    let mut edits = Vec::new();
    for row in rows(buffer, ranges)? {
        let start = buffer.line_start(row)?;
        let text = buffer.line(row)?;
        let end = start + text.chars().count();
        let text = if text.ends_with('\n') {
            text
        } else {
            format!("{}{text}", buffer.line_ending.text())
        };
        edits.push(Edit {
            range: end..end,
            text,
        });
    }
    let heads = buffer.apply_edits(&edits)?;
    Ok(heads.into_iter().map(|p| p..p).collect())
}
pub fn move_lines(
    buffer: &mut Buffer,
    ranges: &[Range<usize>],
    up: bool,
) -> Result<Vec<Range<usize>>, EditorError> {
    let selected = rows(buffer, ranges)?;
    let mut blocks: Vec<Range<usize>> = Vec::new();
    for row in selected {
        if let Some(block) = blocks.last_mut()
            && block.end == row
        {
            block.end = row + 1;
        } else {
            blocks.push(row..row + 1);
        }
    }
    let mut edits = Vec::new();
    let positions: Vec<_> = ranges
        .iter()
        .map(|r| Ok((buffer.position(r.start)?, buffer.position(r.end)?)))
        .collect::<Result<_, EditorError>>()?;
    let mut moved = Vec::new();
    for block in blocks {
        if (up && block.start == 0) || (!up && block.end >= buffer.line_count()) {
            continue;
        }
        let first = if up { block.start - 1 } else { block.start };
        let last = if up { block.end } else { block.end + 1 };
        let lines: Vec<_> = (first..last)
            .map(|row| buffer.line(row))
            .collect::<Result<_, _>>()?;
        let mut order: Vec<_> = (0..lines.len()).collect();
        if up {
            order.rotate_left(1);
        } else {
            order.rotate_right(1);
        }
        let mut replacement = String::new();
        for (slot, index) in order.into_iter().enumerate() {
            replacement.push_str(lines[index].trim_end_matches(['\r', '\n']));
            let content = lines[slot].trim_end_matches(['\r', '\n']);
            replacement.push_str(&lines[slot][content.len()..]);
        }
        let start = buffer.line_start(first)?;
        let end = if last < buffer.line_count() {
            buffer.line_start(last)?
        } else {
            buffer.len_chars()
        };
        edits.push(Edit {
            range: start..end,
            text: replacement,
        });
        moved.push(block);
    }
    buffer.apply_edits(&edits)?;
    positions
        .into_iter()
        .map(|((a, ac), (b, bc))| {
            let shift = |row: usize| {
                if moved.iter().any(|r| r.contains(&row)) {
                    if up { row - 1 } else { row + 1 }
                } else {
                    row
                }
            };
            let a = buffer.offset(shift(a), ac)?;
            let b = buffer.offset(shift(b), bc)?;
            Ok(a.min(b)..a.max(b))
        })
        .collect()
}
pub fn word_range(buffer: &Buffer, offset: usize) -> Range<usize> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut a = offset.min(buffer.len_chars());
    let mut b = a;
    while a > 0 && buffer.rope().get_char(a - 1).is_some_and(word) {
        a -= 1;
    }
    while b < buffer.len_chars() && buffer.rope().get_char(b).is_some_and(word) {
        b += 1;
    }
    a..b
}
pub fn next_occurrence(buffer: &Buffer, ranges: &[Range<usize>]) -> Option<Range<usize>> {
    let ranges = normalized(ranges);
    let chosen = ranges.first()?;
    let chosen = if chosen.is_empty() {
        word_range(buffer, chosen.start)
    } else {
        chosen.clone()
    };
    let needle: Vec<char> = buffer.slice(chosen.clone()).ok()?.chars().collect();
    if needle.is_empty() || needle.len() > 4096 {
        return None;
    }
    if ranges.iter().all(Range::is_empty) {
        return Some(chosen);
    }
    let mut prefix = vec![0; needle.len()];
    for i in 1..needle.len() {
        let mut j = prefix[i - 1];
        while j > 0 && needle[i] != needle[j] {
            j = prefix[j - 1];
        }
        if needle[i] == needle[j] {
            j += 1;
        }
        prefix[i] = j;
    }
    let end = ranges.iter().map(|r| r.end).max()?;
    let mut budget = 250_000;
    for (start, end) in [(end, buffer.len_chars()), (0, chosen.start)] {
        let mut j = 0;
        for (index, c) in buffer.rope().chars_at(start).take(end - start).enumerate() {
            if budget == 0 {
                return None;
            }
            budget -= 1;
            while j > 0 && c != needle[j] {
                j = prefix[j - 1];
            }
            if c == needle[j] {
                j += 1;
            }
            if j == needle.len() {
                let range = start + index + 1 - needle.len()..start + index + 1;
                if !ranges
                    .iter()
                    .any(|r| r.start < range.end && r.end > range.start)
                {
                    return Some(range);
                }
                j = prefix[j - 1];
            }
        }
    }
    None
}
fn ignored(buffer: &Buffer, position: usize, parsed: Option<&Parsed>) -> bool {
    let Some(parsed) = parsed else {
        return false;
    };
    let byte = buffer.char_to_byte(position).unwrap_or(0);
    let index = parsed.spans.partition_point(|s| s.bytes.end <= byte);
    parsed
        .spans
        .get(index)
        .is_some_and(|s| s.bytes.contains(&byte) && matches!(s.kind, Kind::String | Kind::Comment))
}
pub fn matching_bracket(
    buffer: &Buffer,
    caret: usize,
    parsed: Option<&Parsed>,
) -> Option<(usize, usize)> {
    let bracket = |c: char| "()[]{}".contains(c);
    let at = if buffer.rope().get_char(caret).is_some_and(bracket) {
        caret
    } else {
        caret.checked_sub(1)?
    };
    let c = buffer.rope().get_char(at)?;
    if !bracket(c) || ignored(buffer, at, parsed) {
        return None;
    }
    let forward = "([{".contains(c);
    let pair = |c: char| match c {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => '\0',
    };
    let mut stack = vec![c];
    let mut position = at;
    for _ in 0..250_000 {
        position = if forward {
            position.checked_add(1)?
        } else {
            position.checked_sub(1)?
        };
        let c = buffer.rope().get_char(position)?;
        if ignored(buffer, position, parsed) {
            continue;
        }
        if !bracket(c) {
            continue;
        }
        if (forward && "([{".contains(c)) || (!forward && ")]}".contains(c)) {
            stack.push(c);
        } else {
            if pair(*stack.last()?) != c {
                return None;
            }
            stack.pop();
            if stack.is_empty() {
                return Some((at, position));
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multi_cursor_is_one_undo_transaction() {
        let mut b = Buffer::from_text("é x\né x\n");
        insert(&mut b, &[0..0, 4..4], "🦀", false).expect("insert");
        assert_eq!(b.rope().to_string(), "🦀é x\n🦀é x\n");
        assert!(b.undo());
        assert_eq!(b.rope().to_string(), "é x\né x\n");
        assert!(
            b.apply_edits(&[
                Edit {
                    range: 0..1,
                    text: "a".into()
                },
                Edit {
                    range: 99..100,
                    text: "b".into()
                }
            ])
            .is_err()
        );
        assert_eq!(b.rope().to_string(), "é x\né x\n");
    }
    #[test]
    fn line_moves_preserve_last_line_and_crlf() {
        let mut b = Buffer::from_text("one\r\nlast");
        move_lines(&mut b, std::slice::from_ref(&(5..5)), true).expect("move");
        assert_eq!(b.rope().to_string(), "last\r\none");
        assert!(b.undo());
        let len = b.len_chars();
        comment(&mut b, std::slice::from_ref(&(0..len)), "//").expect("comment");
        let len = b.len_chars();
        comment(&mut b, std::slice::from_ref(&(0..len)), "//").expect("uncomment");
        assert_eq!(b.rope().to_string(), "one\r\nlast");
    }
    #[test]
    fn pairing_occurrences_and_matching() {
        let mut b = Buffer::from_text("name name");
        assert_eq!(
            next_occurrence(&b, std::slice::from_ref(&(0..4))),
            Some(5..9)
        );
        insert(&mut b, std::slice::from_ref(&(0..4)), "(", true).expect("pair");
        assert_eq!(matching_bracket(&b, 0, None), Some((0, 5)));
        assert_eq!(b.rope().to_string(), "(name) name");
    }
}
