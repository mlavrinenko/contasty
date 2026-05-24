//! Markdown rendering of stripped files.

use std::fmt::Write;

use crate::walk::Stripped;

/// Render a list of stripped files as a single Markdown document.
///
/// Each file becomes an `## <path>` heading followed by a fenced code block
/// tagged with the language name.
#[must_use]
pub fn render_markdown(items: &[Stripped]) -> String {
    let mut out = String::new();
    for item in items {
        let _ = write!(
            out,
            "## {path}\n\n```{lang}\n{body}\n```\n\n",
            path = item.path.display(),
            lang = item.lang_name,
            body = item.content.trim_end(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn render_emits_path_heading_and_fenced_block() {
        let items = vec![Stripped {
            path: PathBuf::from("src/lib.rs"),
            lang_name: "rust",
            content: "pub fn greet() { /* ... */ }".to_owned(),
        }];
        let md = render_markdown(&items);
        assert!(md.contains("## src/lib.rs"), "missing path heading: {md}");
        assert!(md.contains("```rust"), "missing fence: {md}");
        assert!(
            md.contains("pub fn greet() { /* ... */ }"),
            "missing body: {md}"
        );
        assert!(md.contains("\n```\n"), "missing closing fence: {md}");
    }

    #[test]
    fn render_with_no_items_yields_empty_string() {
        assert_eq!(render_markdown(&[]), "");
    }

    #[test]
    fn render_separates_files_with_blank_lines() {
        let items = vec![
            Stripped {
                path: PathBuf::from("a.rs"),
                lang_name: "rust",
                content: "fn a() { /* ... */ }".to_owned(),
            },
            Stripped {
                path: PathBuf::from("b.rs"),
                lang_name: "rust",
                content: "fn b() { /* ... */ }".to_owned(),
            },
        ];
        let md = render_markdown(&items);
        let a_pos = md.find("## a.rs").expect("a heading");
        let b_pos = md.find("## b.rs").expect("b heading");
        assert!(a_pos < b_pos);
        assert!(
            md[a_pos..b_pos].contains("```\n\n"),
            "no blank line between files: {md}"
        );
    }
}
