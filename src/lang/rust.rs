//! Rust language support.
//!
//! Strategy: keep every top-level item's signature, elide bodies that contain
//! runtime logic. The tree-sitter query below captures the `block` node that
//! follows any `fn` signature (free functions and `impl` methods both desugar
//! to `function_item` in the grammar). Trait method declarations without a
//! body are `function_signature_item` nodes and are kept verbatim.
//!
//! Test items annotated with `#[test]` or `#[cfg(test)]` are matched by a
//! second query and removed entirely when the caller opts to drop tests.
//!
//! A third query matches every `line_comment` and `block_comment` — including
//! doc comments — and removes them when the caller opts to drop comments.

use tree_sitter::Query;

use crate::AppError;

use super::Language;

const EXTENSIONS: &[&str] = &["rs"];

/// Nodes captured here are replaced with {ELISION}.
const ELIDE_QUERY: &str = r"
(function_item body: (block) @elide)
";

/// Captures `const_item` value expressions for elision.
const CONST_ELIDE_QUERY: &str = r"
(const_item value: (_) @elide)
";

/// Captures `static_item` value expressions for elision.
const STATIC_ELIDE_QUERY: &str = r"
(static_item value: (_) @elide)
";

/// Captures `type_item` type expressions for elision.
const TYPE_ELIDE_QUERY: &str = r"
(type_item type: (_) @elide)
";

/// Captures `string_literal` and `raw_string_literal` nodes for truncation.
const STRING_TRIM_QUERY: &str = r"
(string_literal) @trim
(raw_string_literal) @trim
";

/// Matches any `#[test]` or `#[cfg(test)]` `attribute_item`. The splicer walks
/// the AST from the captured attribute to absorb adjacent attribute siblings
/// and the item they decorate — so `#[cfg(test)] #[allow(...)] mod tests {}`
/// gets removed as one block.
///
/// Note: backslashes are doubled — the Rust raw string passes them verbatim,
/// then tree-sitter's query string parser un-escapes once into the regex.
const TEST_QUERY: &str = r#"
((attribute_item) @attr
 (#match? @attr "^#\\[(test|cfg\\(test\\))\\]$"))
"#;

/// Every `//`, `///`, `//!` line comment and every `/* */`, `/** */`, `/*! */`
/// block comment — tree-sitter-rust does not distinguish doc from non-doc at
/// the node level, and the caller asked for all-or-nothing.
const COMMENT_QUERY: &str = r"
(line_comment) @comment
(block_comment) @comment
";

/// Every `use` declaration. Removed entirely when the caller opts to drop
/// imports — an import list rarely helps an LLM grasp a file's structure.
const IMPORT_QUERY: &str = r"
(use_declaration) @import
";

/// Build the Rust language descriptor.
///
/// # Errors
///
/// Returns [`AppError::Query`] if any embedded query fails to compile against
/// the bundled tree-sitter-rust grammar. This is a programming error, not a
/// runtime condition.
pub fn language() -> Result<Language, AppError> {
    let grammar = tree_sitter_rust::language();
    let elide_query = Query::new(grammar, ELIDE_QUERY)?;
    let const_elide_query = Some(Query::new(grammar, CONST_ELIDE_QUERY)?);
    let static_elide_query = Some(Query::new(grammar, STATIC_ELIDE_QUERY)?);
    let type_elide_query = Some(Query::new(grammar, TYPE_ELIDE_QUERY)?);
    let string_trim_query = Some(Query::new(grammar, STRING_TRIM_QUERY)?);
    let test_query = Query::new(grammar, TEST_QUERY)?;
    let comment_query = Query::new(grammar, COMMENT_QUERY)?;
    let import_query = Query::new(grammar, IMPORT_QUERY)?;
    Ok(Language {
        name: "rust",
        extensions: EXTENSIONS,
        grammar,
        elide_query,
        const_elide_query,
        static_elide_query,
        type_elide_query,
        string_trim_query,
        test_query,
        comment_query,
        import_query,
        format: Some(format),
    })
}

/// Pretty-print stripped Rust source via `prettyplease`. Returns `None` if
/// `syn` cannot parse the input — caller falls back to the unformatted text.
fn format(source: &str) -> Option<String> {
    let file = syn::parse_file(source).ok()?;
    Some(prettyplease::unparse(&file))
}
