//! Markdown rendering of stripped files.

use std::fmt::Write;
use std::path::PathBuf;

use crate::walk::Stripped;

fn common_base(paths: &[PathBuf]) -> PathBuf {
    let Some(first) = paths.first() else {
        return PathBuf::new();
    };
    let mut base = first.clone();
    let iter = paths.iter().skip(1);
    for path in iter {
        while !path.starts_with(&base) || base == *path {
            if !base.pop() {
                return PathBuf::new();
            }
        }
    }
    if paths.len() == 1 {
        base.pop();
    }
    if base.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        base
    }
}

/// Render a list of stripped files as a single Markdown document.
///
/// Output starts with a level-1 heading showing the shared base directory,
/// followed by one level-2 heading per file using a path relative to that base.
#[must_use]
pub fn render_markdown(items: &[Stripped]) -> String {
    let mut out = String::new();
    if items.is_empty() {
        return out;
    }

    let paths: Vec<_> = items.iter().map(|i| i.path.clone()).collect();
    let base = common_base(&paths);

    let _ = writeln!(out, "# {}\n", base.display());

    for item in items {
        let rel = item.path.strip_prefix(&base).unwrap_or(&item.path);
        let _ = write!(
            out,
            "## {path}\n\n```{lang}\n{body}\n```\n\n",
            path = rel.display(),
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
    fn render_emits_base_heading_and_relative_paths() {
        let items = vec![
            Stripped {
                path: PathBuf::from("src/lib.rs"),
                lang_name: "rust",
                original: String::new(),
                content: "pub fn greet() { /* ... */ }".to_owned(),
            },
            Stripped {
                path: PathBuf::from("src/main.rs"),
                lang_name: "rust",
                original: String::new(),
                content: "fn main() { /* ... */ }".to_owned(),
            },
        ];
        let md = render_markdown(&items);
        assert!(md.starts_with("# src\n"), "missing base heading: {md}");
        assert!(md.contains("## lib.rs"), "missing relative heading: {md}");
        assert!(md.contains("## main.rs"), "missing relative heading: {md}");
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
                original: String::new(),
                content: "fn a() { /* ... */ }".to_owned(),
            },
            Stripped {
                path: PathBuf::from("b.rs"),
                lang_name: "rust",
                original: String::new(),
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

    #[test]
    fn common_base_returns_dot_for_empty() {
        assert_eq!(common_base(&[]), PathBuf::new());
    }

    #[test]
    fn common_base_single_path_returns_parent() {
        let paths = vec![PathBuf::from("src/lib.rs")];
        assert_eq!(common_base(&paths), PathBuf::from("src"));
    }

    #[test]
    fn common_base_finds_shared_prefix() {
        let paths = vec![
            PathBuf::from("src/lang/mod.rs"),
            PathBuf::from("src/lang/rust.rs"),
            PathBuf::from("src/walk.rs"),
        ];
        assert_eq!(common_base(&paths), PathBuf::from("src"));
    }
}
