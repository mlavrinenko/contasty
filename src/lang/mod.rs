//! Language registry and source-stripping core.
//!
//! Adding a language is a three-step recipe:
//!
//! 1. Add a `tree-sitter-<lang>` dependency.
//! 2. Drop a sibling module here that returns a [`Language`] (grammar + elide
//!    query + file extensions).
//! 3. Register it inside [`Registry::new`].
//!
//! Everything else — extension dispatch, parsing, byte-range splicing — is
//! language-agnostic and lives in this module.

use std::path::Path;

use tree_sitter::{Parser, Query, QueryCursor};

use crate::AppError;

mod rust;

/// A registered language: grammar + tree-sitter query identifying elidable nodes.
pub struct Language {
    /// Markdown fence info-string (e.g. `"rust"`).
    pub name: &'static str,
    extensions: &'static [&'static str],
    grammar: tree_sitter::Language,
    elide_query: Query,
}

impl Language {
    /// Strip elidable nodes from `source`, returning the trimmed text.
    ///
    /// # Errors
    ///
    /// - [`AppError::LangLoad`] if tree-sitter rejects the grammar.
    /// - [`AppError::ParseFailed`] if tree-sitter cannot produce a parse tree.
    pub fn strip(&self, source: &str, path: &Path) -> Result<String, AppError> {
        let mut parser = Parser::new();
        parser.set_language(self.grammar)?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| AppError::ParseFailed {
                path: path.to_path_buf(),
            })?;
        let ranges = collect_ranges(&mut QueryCursor::new(), &self.elide_query, &tree, source);
        Ok(splice_elide(source, &ranges))
    }
}

fn collect_ranges(
    cursor: &mut QueryCursor,
    query: &Query,
    tree: &tree_sitter::Tree,
    source: &str,
) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for mat in cursor.matches(query, tree.root_node(), source.as_bytes()) {
        for cap in mat.captures {
            let node = cap.node;
            ranges.push((node.start_byte(), node.end_byte()));
        }
    }
    ranges
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

fn splice_elide(source: &str, ranges: &[(usize, usize)]) -> String {
    let sorted = sort_dedup(ranges);
    if sorted.is_empty() {
        return source.to_owned();
    }
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0_usize;
    for (start, end) in sorted {
        if start < cursor {
            continue;
        }
        out.push_str(source.get(cursor..start).unwrap_or_default());
        out.push_str(ELISION);
        cursor = end;
    }
    out.push_str(source.get(cursor..).unwrap_or_default());
    out
}

fn sort_dedup(ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut sorted: Vec<(usize, usize)> = ranges.to_vec();
    sorted.sort_by_key(|&(start, _)| start);
    sorted.dedup();
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice_replaces_two_function_bodies() {
        let src = "fn a() { foo(); }\nfn b() { bar(); }\n";
        let ranges = vec![(7, 17), (25, 35)];
        let out = splice_elide(src, &ranges);
        assert_eq!(out, "fn a() { /* ... */ }\nfn b() { /* ... */ }\n");
    }

    #[test]
    fn splice_with_no_ranges_returns_source() {
        let src = "hello world";
        assert_eq!(splice_elide(src, &[]), src);
    }

    #[test]
    fn splice_drops_overlapping_ranges() {
        let src = "abcdef";
        let out = splice_elide(src, &[(1, 4), (2, 5)]);
        assert_eq!(out, "a{ /* ... */ }ef");
    }

    #[test]
    fn splice_handles_unsorted_input() {
        let src = "fn a() { foo(); }\nfn b() { bar(); }\n";
        let ranges = vec![(25, 35), (7, 17)];
        let out = splice_elide(src, &ranges);
        assert_eq!(out, "fn a() { /* ... */ }\nfn b() { /* ... */ }\n");
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
            )
            .expect("strip");
        assert!(stripped.contains("fn add(lhs: i32, rhs: i32) -> i32"));
        assert!(stripped.contains("/* ... */"));
        assert!(!stripped.contains("lhs + rhs"));
    }
}
