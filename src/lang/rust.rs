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

use tree_sitter::Query;

use crate::AppError;

use super::Language;

const EXTENSIONS: &[&str] = &["rs"];

/// Nodes captured here are replaced with `{ /* ... */ }`.
const ELIDE_QUERY: &str = r"
(function_item body: (block) @elide)
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

/// Build the Rust language descriptor.
///
/// # Errors
///
/// Returns [`AppError::Query`] if `ELIDE_QUERY` or `TEST_QUERY` fail to compile
/// against the bundled tree-sitter-rust grammar. This is a programming error,
/// not a runtime condition.
pub fn language() -> Result<Language, AppError> {
    let grammar = tree_sitter_rust::language();
    let elide_query = Query::new(grammar, ELIDE_QUERY)?;
    let test_query = Query::new(grammar, TEST_QUERY)?;
    Ok(Language {
        name: "rust",
        extensions: EXTENSIONS,
        grammar,
        elide_query,
        test_query,
    })
}
