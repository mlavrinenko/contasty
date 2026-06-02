//! Byte-range splicing: turn a set of `(start, end, Action)` hits into the
//! stripped source. The matcher (in the parent module) decides *what* to splice;
//! this module owns *how* — overlap resolution, the elision/truncation markers,
//! and line-aware deletion.

use super::Action;

const ELISION: &str = "{}";
const STR_TRUNCATION: &str = "\"[…CTY]\"";

pub(super) fn splice(source: &str, ranges: &[(usize, usize, Action)]) -> String {
    if ranges.is_empty() {
        return source.to_owned();
    }
    let sorted = sort_ranges(ranges);
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0_usize;
    for &(start, end, action) in &sorted {
        if start < cursor {
            continue;
        }
        // A `delete` of a node alone on its line takes the line's indentation
        // with it, so an indented import/comment/test leaves no blank stub.
        let emit_end = match action {
            Action::Delete => line_indent_start(source, start, cursor),
            Action::Elide | Action::TruncateString => start,
        };
        out.push_str(source.get(cursor..emit_end).unwrap_or_default());
        cursor = apply(action, &mut out, source, end);
    }
    out.push_str(source.get(cursor..).unwrap_or_default());
    out
}

fn apply(action: Action, out: &mut String, source: &str, end: usize) -> usize {
    match action {
        Action::Elide => {
            out.push_str(ELISION);
            end
        }
        Action::Delete => consume_trailing_newline(source, end),
        Action::TruncateString => {
            out.push_str(STR_TRUNCATION);
            end
        }
    }
}

/// For a node about to be deleted, return where to stop emitting preserved
/// text: the start of the node's indentation when only horizontal whitespace
/// precedes it on the line (so the whole line drops), else `start` unchanged.
/// Never rewinds before `floor` (the splice cursor) and only collapses when the
/// run reaches a line boundary, so a trailing same-line node (`code  # note`)
/// keeps its leading space.
fn line_indent_start(source: &str, start: usize, floor: usize) -> usize {
    let bytes = source.as_bytes();
    let mut pos = start;
    while pos > floor {
        match bytes.get(pos - 1) {
            Some(b' ' | b'\t') => pos -= 1,
            Some(b'\n') => return pos,
            _ => return start,
        }
    }
    // Ran back to the cursor over whitespace only: collapse when the cursor sits
    // at a line start (file start, or the byte after a newline), else keep.
    if pos == 0 || bytes.get(pos - 1) == Some(&b'\n') {
        pos
    } else {
        start
    }
}

fn consume_trailing_newline(source: &str, end: usize) -> usize {
    if source.as_bytes().get(end) == Some(&b'\n') {
        end + 1
    } else {
        end
    }
}

fn sort_ranges(ranges: &[(usize, usize, Action)]) -> Vec<(usize, usize, Action)> {
    let mut sorted: Vec<_> = ranges.to_vec();
    // Sort by start ascending, then by end descending so a wider range that
    // shares a start wins over a narrower one (the narrower is skipped via
    // `start < cursor`).
    sorted.sort_by(|left, right| left.0.cmp(&right.0).then(right.1.cmp(&left.1)));
    sorted.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);
    sorted
}
