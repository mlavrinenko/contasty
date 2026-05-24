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
    /// Captures whose ranges become `{ /* ... */ }` (function bodies).
    elide_query: Query,
    /// Captures whose ranges become `{ /* ... */ }` (const value expressions).
    const_elide_query: Option<Query>,
    /// Captures whose ranges become `{ /* ... */ }` (static value expressions).
    static_elide_query: Option<Query>,
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
    /// Replace with `{ /* ... */ }`.
    Elide,
    /// Remove the range plus one trailing newline if present.
    Delete,
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
    #[allow(clippy::too_many_arguments)]
    pub fn strip(
        &self,
        source: &str,
        path: &Path,
        drop_tests: bool,
        drop_comments: bool,
        min_elide_bytes: usize,
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
                min_elide_bytes,
                &mut ranges,
            );
        }
        if let Some(ref q) = self.static_elide_query {
            collect_ranges(
                q,
                &tree,
                source,
                Action::Elide,
                min_elide_bytes,
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

const ELISION: &str = "{ /* ... */ }";

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
    out
}

fn apply(action: Action, out: &mut String, source: &str, end: usize) -> usize {
    match action {
        Action::Elide => {
            out.push_str(ELISION);
            end
        }
        Action::Delete => consume_trailing_newline(source, end),
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
mod tests {
    use super::*;

    #[test]
    fn splice_replaces_two_function_bodies() {
        let src = "fn a() { foo(); }\nfn b() { bar(); }\n";
        let ranges = vec![(7, 17, Action::Elide), (25, 35, Action::Elide)];
        let out = splice(src, &ranges);
        assert_eq!(out, "fn a() { /* ... */ }\nfn b() { /* ... */ }\n");
    }

    #[test]
    fn splice_with_no_ranges_returns_source() {
        let src = "hello world";
        assert_eq!(splice(src, &[]), src);
    }

    #[test]
    fn splice_drops_overlapping_ranges() {
        let src = "abcdef";
        let out = splice(src, &[(1, 4, Action::Elide), (2, 5, Action::Elide)]);
        assert_eq!(out, "a{ /* ... */ }ef");
    }

    #[test]
    fn splice_handles_unsorted_input() {
        let src = "fn a() { foo(); }\nfn b() { bar(); }\n";
        let ranges = vec![(25, 35, Action::Elide), (7, 17, Action::Elide)];
        let out = splice(src, &ranges);
        assert_eq!(out, "fn a() { /* ... */ }\nfn b() { /* ... */ }\n");
    }

    #[test]
    fn splice_delete_action_removes_range_and_trailing_newline() {
        let src = "keep\n#[cfg(test)]\nmod t {}\nkeep\n";
        // Delete the attr+mod range — bytes 5..26 covers "#[cfg(test)]\nmod t {}".
        let out = splice(src, &[(5, 26, Action::Delete)]);
        assert_eq!(out, "keep\nkeep\n");
    }

    #[test]
    fn detect_matches_known_extension() {
        let reg = Registry::new().expect("registry init");
        assert!(reg.detect(Path::new("foo.rs")).is_some());
        assert!(reg.detect(Path::new("foo.py")).is_none());
        assert!(reg.detect(Path::new("noext")).is_none());
    }

    #[test]
    fn registry_strips_a_rust_file() {
        let reg = Registry::new().expect("registry init");
        let lang = reg.detect(Path::new("x.rs")).expect("rust");
        let stripped = lang
            .strip(
                "fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n",
                Path::new("x.rs"),
                false,
                false,
                0,
            )
            .expect("strip");
        assert!(stripped.contains("fn add(lhs: i32, rhs: i32) -> i32"));
        assert!(stripped.contains("/* ... */"));
        assert!(!stripped.contains("lhs + rhs"));
    }

    #[test]
    fn drop_tests_removes_cfg_test_module() {
        let reg = Registry::new().expect("registry init");
        let lang = reg.detect(Path::new("x.rs")).expect("rust");
        let src = "pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n\n\
                   #[cfg(test)]\n\
                   mod tests {\n    \
                       use super::*;\n    \
                       #[test]\n    \
                       fn it_adds() { assert_eq!(add(1, 2), 3); }\n\
                   }\n";
        let stripped = lang
            .strip(src, Path::new("x.rs"), true, false, 0)
            .expect("strip");
        assert!(stripped.contains("pub fn add"));
        assert!(
            !stripped.contains("cfg(test)"),
            "cfg(test) attribute remained: {stripped}"
        );
        assert!(
            !stripped.contains("mod tests"),
            "test module remained: {stripped}"
        );
        assert!(
            !stripped.contains("it_adds"),
            "test fn remained: {stripped}"
        );
    }

    #[test]
    fn keep_tests_keeps_cfg_test_module() {
        let reg = Registry::new().expect("registry init");
        let lang = reg.detect(Path::new("x.rs")).expect("rust");
        let src = "pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n\n\
                   #[cfg(test)]\n\
                   mod tests {\n    \
                       #[test]\n    \
                       fn it_adds() { assert_eq!(add(1, 2), 3); }\n\
                   }\n";
        let stripped = lang
            .strip(src, Path::new("x.rs"), false, false, 0)
            .expect("strip");
        assert!(stripped.contains("mod tests"));
        assert!(stripped.contains("fn it_adds"));
        assert!(stripped.contains("/* ... */"));
    }

    #[test]
    fn drop_tests_removes_top_level_test_function() {
        let reg = Registry::new().expect("registry init");
        let lang = reg.detect(Path::new("x.rs")).expect("rust");
        let src = "pub fn keep() {}\n\n#[test]\nfn freestanding() { assert!(true); }\n";
        let stripped = lang
            .strip(src, Path::new("x.rs"), true, false, 0)
            .expect("strip");
        assert!(stripped.contains("pub fn keep"));
        assert!(!stripped.contains("freestanding"));
        assert!(!stripped.contains("#[test]"));
    }

    #[test]
    fn drop_tests_absorbs_other_attributes_on_the_test_module() {
        let reg = Registry::new().expect("registry init");
        let lang = reg.detect(Path::new("x.rs")).expect("rust");
        let src = "pub fn keep() {}\n\n\
                   #[cfg(test)]\n\
                   #[allow(clippy::unwrap_used)]\n\
                   mod tests {\n    \
                       fn helper() {}\n\
                   }\n";
        let stripped = lang
            .strip(src, Path::new("x.rs"), true, false, 0)
            .expect("strip");
        assert!(stripped.contains("pub fn keep"));
        assert!(
            !stripped.contains("mod tests"),
            "test mod remained: {stripped}"
        );
        assert!(
            !stripped.contains("allow(clippy::unwrap_used)"),
            "orphan attribute remained: {stripped}",
        );
    }

    #[test]
    fn drop_comments_removes_line_block_and_doc_comments() {
        let reg = Registry::new().expect("registry init");
        let lang = reg.detect(Path::new("x.rs")).expect("rust");
        let src = "// regular line comment\n\
                   /// outer doc\n\
                   //! inner doc\n\
                   /* block */\n\
                   /** outer block doc */\n\
                   /*! inner block doc */\n\
                   pub fn keep() {}\n";
        let stripped = lang
            .strip(src, Path::new("x.rs"), false, true, 0)
            .expect("strip");
        assert!(stripped.contains("pub fn keep"));
        assert!(
            !stripped.contains("regular line comment"),
            "line comment remained: {stripped}"
        );
        assert!(
            !stripped.contains("outer doc"),
            "/// doc comment remained: {stripped}"
        );
        assert!(
            !stripped.contains("inner doc"),
            "//! doc comment remained: {stripped}"
        );
        assert!(
            !stripped.contains("block"),
            "/* */ block comment remained: {stripped}"
        );
        assert!(
            !stripped.contains("outer block doc"),
            "/** */ block doc remained: {stripped}"
        );
        assert!(
            !stripped.contains("inner block doc"),
            "/*! */ block doc remained: {stripped}"
        );
    }

    #[test]
    fn keep_comments_keeps_everything() {
        let reg = Registry::new().expect("registry init");
        let lang = reg.detect(Path::new("x.rs")).expect("rust");
        let src = "/// doc\npub fn keep() {}\n// trailing\n";
        let stripped = lang
            .strip(src, Path::new("x.rs"), false, false, 0)
            .expect("strip");
        assert!(stripped.contains("/// doc"));
        assert!(stripped.contains("// trailing"));
    }
}
