//! Rust language support.
//!
//! Strategy: keep every top-level item's signature, elide bodies that contain
//! runtime logic. The tree-sitter query below captures the `block` node that
//! follows any `fn` signature (free functions and `impl` methods both desugar
//! to `function_item` in the grammar). Trait method declarations without a
//! body are `function_signature_item` nodes and are kept verbatim.

use tree_sitter::Query;

use crate::AppError;

use super::Language;

const EXTENSIONS: &[&str] = &["rs"];

/// Tree-sitter query: every node captured as `@elide` is replaced with `{ /* ... */ }`.
const ELIDE_QUERY: &str = r"
(function_item body: (block) @elide)
";

/// Build the Rust language descriptor.
///
/// # Errors
///
/// Returns [`AppError::Query`] if `ELIDE_QUERY` fails to compile against the
/// bundled tree-sitter-rust grammar. This is a programming error, not a
/// runtime condition.
pub fn language() -> Result<Language, AppError> {
    let grammar = tree_sitter_rust::language();
    let elide_query = Query::new(grammar, ELIDE_QUERY)?;
    Ok(Language {
        name: "rust",
        extensions: EXTENSIONS,
        grammar,
        elide_query,
    })
}
