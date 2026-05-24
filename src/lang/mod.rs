//! Language registry and source-stripping core.
//!
//! Adding a language is a three-step recipe:
//!
//! 1. Add a `tree-sitter-<lang>` dependency.
//! 2. Drop a sibling module here that returns a [`Language`] (grammar + elide
//!    query + test-drop query + file extensions).
//! 3. Register it inside [`Registry::new`].
//!
//! Everything else — extension dispatch, parsing, byte-range splicing — is
//! language-agnostic and lives in this module.

use std::path::Path;

use tree_sitter::{Parser, Query, QueryCursor};

use crate::AppError;

mod rust;

/// A registered language: grammar + tree-sitter queries.
pub struct Language {
    /// Markdown fence info-string (e.g. `"rust"`).
    pub name: &'static str,
    extensions: &'static [&'static str],
    grammar: tree_sitter::Language,
    /// Captures whose ranges become `ELISION` (function bodies).
    elide_query: Query,
    /// Captures whose ranges become `ELISION` (const value expressions).
    const_elide_query: Option<Query>,
    /// Captures whose ranges become `ELISION` (static value expressions).
    static_elide_query: Option<Query>,
    /// Captures whose ranges become `ELISION` (type alias values).
    type_elide_query: Option<Query>,
    /// Captures whose ranges become `STR_TRUNCATION` (string literals).
    string_trim_query: Option<Query>,
    /// Captures whose ranges are removed entirely. Each match's captures are
    /// merged into one range so attribute + item collapse together.
    test_query: Query,
    /// Captures whose ranges are removed entirely (comments). No attribute
    /// expansion — each capture stands alone.
    comment_query: Query,
}

/// What to do with a captured byte range.
#[derive(Clone, Copy)]
enum Action {
    /// Replace with `ELISION}`.
    Elide,
    /// Remove the range plus one trailing newline if present.
    Delete,
    /// Replace with a string-truncation marker.
    TruncateString,
}

impl Language {
    /// Strip elidable nodes from `source`. When `drop_tests` is true, test
    /// items (`#[test]` / `#[cfg(test)]`) are removed entirely. When
    /// `drop_comments` is true, every `line_comment` and `block_comment` is
    /// removed (doc comments included — the caller asked for all-or-nothing).
    ///
    /// # Errors
    ///
    /// - [`AppError::LangLoad`] if tree-sitter rejects the grammar.
    /// - [`AppError::ParseFailed`] if tree-sitter cannot produce a parse tree.
    #[allow(clippy::too_many_arguments)]
    pub fn strip(
        &self,
        source: &str,
        path: &Path,
        drop_tests: bool,
        drop_comments: bool,
        compact: &crate::config::CompactConfig,
    ) -> Result<String, AppError> {
        let mut parser = Parser::new();
        parser.set_language(self.grammar)?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| AppError::ParseFailed {
                path: path.to_path_buf(),
            })?;
        let mut ranges = Vec::new();
        collect_ranges(
            &self.elide_query,
            &tree,
            source,
            Action::Elide,
            0,
            &mut ranges,
        );
        if let Some(ref q) = self.const_elide_query {
            collect_ranges(
                q,
                &tree,
                source,
                Action::Elide,
                compact.elide_min_bytes,
                &mut ranges,
            );
        }
        if let Some(ref q) = self.static_elide_query {
            collect_ranges(
                q,
                &tree,
                source,
                Action::Elide,
                compact.elide_min_bytes,
                &mut ranges,
            );
        }
        if let Some(ref q) = self.type_elide_query {
            collect_ranges(
                q,
                &tree,
                source,
                Action::Elide,
                compact.elide_min_bytes,
                &mut ranges,
            );
        }
        if let Some(ref q) = self.string_trim_query {
            collect_ranges(
                q,
                &tree,
                source,
                Action::TruncateString,
                compact.max_string_bytes,
                &mut ranges,
            );
        }
        if drop_tests {
            collect_tests(&self.test_query, &tree, source, &mut ranges);
        }
        if drop_comments {
            collect_ranges(
                &self.comment_query,
                &tree,
                source,
                Action::Delete,
                0,
                &mut ranges,
            );
        }
        Ok(splice(source, &ranges))
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_ranges(
    query: &Query,
    tree: &tree_sitter::Tree,
    source: &str,
    action: Action,
    min_bytes: usize,
    out: &mut Vec<(usize, usize, Action)>,
) {
    let mut cursor = QueryCursor::new();
    for mat in cursor.matches(query, tree.root_node(), source.as_bytes()) {
        for cap in mat.captures {
            let node = cap.node;
            let start = node.start_byte();
            let end = node.end_byte();
            if end - start >= min_bytes {
                out.push((start, end, action));
            }
        }
    }
}

fn collect_tests(
    query: &Query,
    tree: &tree_sitter::Tree,
    source: &str,
    out: &mut Vec<(usize, usize, Action)>,
) {
    let mut cursor = QueryCursor::new();
    for mat in cursor.matches(query, tree.root_node(), source.as_bytes()) {
        for cap in mat.captures {
            let (start, end) = expand_attribute_to_item(cap.node);
            if start < end {
                out.push((start, end, Action::Delete));
            }
        }
    }
}

/// Given an `attribute_item` node, walk to its sibling item, absorbing any
/// other `attribute_item` siblings along the way. Returns the byte range
/// covering the whole `#[a] #[b] item` group.
fn expand_attribute_to_item(attr: tree_sitter::Node) -> (usize, usize) {
    let mut leftmost = attr;
    while let Some(prev) = leftmost.prev_named_sibling() {
        if prev.kind() != "attribute_item" {
            break;
        }
        leftmost = prev;
    }
    let mut rightmost = attr;
    while let Some(next) = rightmost.next_named_sibling() {
        rightmost = next;
        if next.kind() != "attribute_item" {
            break;
        }
    }
    (leftmost.start_byte(), rightmost.end_byte())
}

/// Set of languages contasty knows how to strip.
pub struct Registry {
    langs: Vec<Language>,
}

impl Registry {
    /// Build the registry with every supported language.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Query`] if any embedded query fails to compile
    /// against its grammar. This is effectively a build-time bug.
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            langs: vec![rust::language()?],
        })
    }

    /// Detect a language from a path's file extension.
    #[must_use]
    pub fn detect(&self, path: &Path) -> Option<&Language> {
        let ext = path.extension()?.to_str()?;
        self.langs.iter().find(|lg| lg.extensions.contains(&ext))
    }
}

const ELISION: &str = "{}";
const STR_TRUNCATION: &str = "\"[…CTY]\"";

fn splice(source: &str, ranges: &[(usize, usize, Action)]) -> String {
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
        out.push_str(source.get(cursor..start).unwrap_or_default());
        cursor = apply(action, &mut out, source, end);
    }
    out.push_str(source.get(cursor..).unwrap_or_default());
    compact_blanks(&mut out);
    out
}

fn compact_blanks(source: &mut String) {
    let bytes = source.as_bytes();
    let mut result = String::with_capacity(source.len());
    let mut pos = 0;
    while pos < bytes.len() {
        let Some(&byte) = bytes.get(pos) else {
            break;
        };
        if byte == b'\n' {
            let start = pos;
            while pos < bytes.len() && bytes.get(pos) == Some(&b'\n') {
                pos += 1;
            }
            let count = pos - start;
            if count > 2 {
                result.push('\n');
                result.push('\n');
            } else {
                for _ in 0..count {
                    result.push('\n');
                }
            }
        } else {
            result.push(byte as char);
            pos += 1;
        }
    }
    *source = result;
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

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
