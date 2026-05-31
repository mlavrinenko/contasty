//! Rust language descriptor.
//!
//! All strip rules live in the sibling `rules/rust.yml` file (an ast-grep rule
//! set), embedded at build time. The only Rust-side logic is the post-strip
//! formatter, which `syn`/`prettyplease` provide and no declarative rule can.

/// Embedded ast-grep rule set driving Rust stripping.
pub const RULES: &str = include_str!("rules/rust.yml");

/// Pretty-print stripped Rust source via `prettyplease`. Returns `None` if
/// `syn` cannot parse the input — caller falls back to the unformatted text.
pub fn format(source: &str) -> Option<String> {
    let file = syn::parse_file(source).ok()?;
    Some(prettyplease::unparse(&file))
}
