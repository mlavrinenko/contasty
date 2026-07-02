//! Line-numbered rendering: the default, agent-native view.
//!
//! Where [`super::splice`] produces a clean skeleton with elided bodies replaced
//! by `{}`, this renders each surviving source line verbatim, prefixed with its
//! original 1-based line number (`N: <line>`). A multi-line elided body keeps its
//! opening line untouched (the cut is shifted to the first newline inside it), so
//! its interior lines simply drop out and the gap in the numbering is the span an
//! agent can read back from the file — no marker on real code. Only a cut that
//! lives inside a single line (a one-line body, a mid-line value or string) is
//! replaced in place with a sentinel, since printing it verbatim would leak the
//! very content the strip removes. Blank and fully-stripped lines are omitted.

use std::fmt::Write;

use super::Action;
use super::splice::resolve;

/// Marker left where an elided range begins on a surviving line.
const ELIDE: &str = "…";
/// Marker for a truncated string literal.
const TRUNCATE: &str = "\"…\"";

/// Render `source` under `ranges` as `N: <line>` rows.
pub(super) fn number_lines(source: &str, ranges: &[(usize, usize, Action)]) -> String {
    let effective = keep_opening_line(source, resolve(ranges));
    let mut out = String::with_capacity(source.len());
    let mut offset = 0_usize;
    let mut first = 0_usize;
    for (index, piece) in source.split_inclusive('\n').enumerate() {
        let line_end = offset + piece.len() - usize::from(piece.ends_with('\n'));
        // Ranges are sorted; skip those wholly before this line so each line
        // only scans the ranges that can touch it.
        while effective.get(first).is_some_and(|entry| entry.1 <= offset) {
            first += 1;
        }
        let text = surviving(
            source,
            offset,
            line_end,
            effective.get(first..).unwrap_or_default(),
        );
        if !text.trim().is_empty() {
            let _ = writeln!(out, "{}: {}", index + 1, text.trim_end());
        }
        offset += piece.len();
    }
    out
}

/// Shift each multi-line `Elide` range to begin at its first interior newline, so
/// the opening line survives verbatim and only whole interior lines drop out.
/// Single-line elisions and every delete/truncate keep their exact span — those
/// are the cuts that must be marked in place because verbatim would leak them.
fn keep_opening_line(
    source: &str,
    ranges: Vec<(usize, usize, Action)>,
) -> Vec<(usize, usize, Action)> {
    ranges
        .into_iter()
        .map(|(start, end, action)| match action {
            Action::Elide => source
                .get(start..end)
                .and_then(|span| span.find('\n'))
                .map_or((start, end, action), |rel| (start + rel, end, action)),
            _ => (start, end, action),
        })
        .collect()
}

/// Build the surviving text of the byte span `[ls, ce)`: verbatim outside any
/// range, a single sentinel where a range starts on this line, nothing for the
/// interior of a range that started earlier.
fn surviving(source: &str, ls: usize, ce: usize, ranges: &[(usize, usize, Action)]) -> String {
    let mut text = String::new();
    let mut cursor = ls;
    for &(start, end, action) in ranges {
        if start >= ce {
            break;
        }
        if end <= ls {
            continue;
        }
        let seg_start = start.max(ls);
        if seg_start > cursor {
            text.push_str(source.get(cursor..seg_start).unwrap_or_default());
        }
        if start >= ls {
            match action {
                Action::Elide => text.push_str(ELIDE),
                Action::TruncateString => text.push_str(TRUNCATE),
                Action::Delete => {}
            }
        }
        cursor = end.min(ce).max(cursor);
    }
    if cursor < ce {
        text.push_str(source.get(cursor..ce).unwrap_or_default());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::super::Action;
    use super::number_lines;

    #[test]
    fn numbers_kept_lines_and_drops_blanks() {
        let src = "struct A {\n\n    x: i32,\n}\n";
        // no ranges: every non-blank line kept, blank line 2 dropped.
        let out = number_lines(src, &[]);
        assert_eq!(out, "1: struct A {\n3:     x: i32,\n4: }\n");
    }

    #[test]
    fn multiline_body_keeps_opening_line_verbatim() {
        // `fn f() { .. }` — elide the multi-line body block (`{ a; b; }`).
        let src = "fn f() {\n    a;\n    b;\n}\nfn g() {}\n";
        let brace = src.find('{').expect("brace");
        let close = src.find("}\n").expect("close") + 1;
        let ranges = [(brace, close, Action::Elide)];
        let out = number_lines(src, &ranges);
        // Opening line stays verbatim (real `{`), 2-4 vanish, g kept at line 5.
        assert_eq!(out, "1: fn f() {\n5: fn g() {}\n");
    }

    #[test]
    fn one_line_body_gets_a_marker_so_it_does_not_leak() {
        // `fn id(x) { x }` — the whole body sits on one line; verbatim would
        // print `{ x }`, so an inline cut is marked instead.
        let src = "fn id(x: i32) -> i32 { x }\n";
        let brace = src.find('{').expect("brace");
        let ranges = [(brace, src.len() - 1, Action::Elide)];
        let out = number_lines(src, &ranges);
        assert_eq!(out, "1: fn id(x: i32) -> i32 …\n");
    }

    #[test]
    fn inline_elision_keeps_surrounding_text() {
        // `const X: u8 = 7;` with the value `7` elided mid-line.
        let src = "const X: u8 = 7;\n";
        let value = src.find('7').expect("value");
        let ranges = [(value, value + 1, Action::Elide)];
        let out = number_lines(src, &ranges);
        assert_eq!(out, "1: const X: u8 = …;\n");
    }
}
