//! Rendering of stripped files as Markdown or JSON.

use std::fmt::Write;
use std::path::PathBuf;

use serde::Serialize;

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

/// One file in the JSON bundle: its base-relative path, language, and body.
#[derive(Serialize)]
struct JsonFile<'src> {
    path: String,
    lang: &'src str,
    content: &'src str,
}

/// Top-level JSON bundle mirroring the Markdown layout.
#[derive(Serialize)]
struct JsonBundle<'src> {
    base: String,
    files: Vec<JsonFile<'src>>,
}

/// Render a list of stripped files as a single pretty-printed JSON document.
///
/// Mirrors [`render_markdown`]: `base` holds the shared base directory and each
/// entry in `files` carries a path relative to that base. Empty input yields a
/// valid bundle with an empty `files` array.
#[must_use]
pub fn render_json(items: &[Stripped]) -> String {
    let paths: Vec<_> = items.iter().map(|item| item.path.clone()).collect();
    let base = common_base(&paths);

    let files = items
        .iter()
        .map(|item| {
            let rel = item.path.strip_prefix(&base).unwrap_or(&item.path);
            JsonFile {
                path: rel.display().to_string(),
                lang: item.lang_name,
                content: item.content.trim_end(),
            }
        })
        .collect();

    let bundle = JsonBundle {
        base: base.display().to_string(),
        files,
    };
    serde_json::to_string_pretty(&bundle).unwrap_or_default()
}

/// Render stripped files as the default line-numbered format.
///
/// Each file is a bare relative-path header (a leading `./` trimmed) followed by
/// its `N: <line>` body; files are separated by a blank line. Files that stripped
/// down to nothing are skipped. The line numbers are the original file's, so an
/// agent can read a body straight back from the gap in the numbering.
#[must_use]
pub fn render_lines(items: &[Stripped]) -> String {
    let mut out = String::new();
    for item in items {
        if item.numbered.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        let path = item.path.to_string_lossy();
        let header = path.strip_prefix("./").unwrap_or(&path);
        let _ = writeln!(out, "{header}");
        out.push_str(&item.numbered);
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
                numbered: String::new(),
            },
            Stripped {
                path: PathBuf::from("src/main.rs"),
                lang_name: "rust",
                original: String::new(),
                content: "fn main() { /* ... */ }".to_owned(),
                numbered: String::new(),
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
                numbered: String::new(),
            },
            Stripped {
                path: PathBuf::from("b.rs"),
                lang_name: "rust",
                original: String::new(),
                content: "fn b() { /* ... */ }".to_owned(),
                numbered: String::new(),
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
    fn render_json_emits_base_and_relative_paths() {
        let items = vec![
            Stripped {
                path: PathBuf::from("src/lib.rs"),
                lang_name: "rust",
                original: String::new(),
                content: "pub fn greet() {}".to_owned(),
                numbered: String::new(),
            },
            Stripped {
                path: PathBuf::from("src/main.rs"),
                lang_name: "rust",
                original: String::new(),
                content: "fn main() {}".to_owned(),
                numbered: String::new(),
            },
        ];
        let json = render_json(&items);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        let str_at = |ptr: &str| value.pointer(ptr).and_then(serde_json::Value::as_str);
        assert_eq!(str_at("/base"), Some("src"));
        assert_eq!(str_at("/files/0/path"), Some("lib.rs"));
        assert_eq!(str_at("/files/0/lang"), Some("rust"));
        assert_eq!(str_at("/files/1/path"), Some("main.rs"));
    }

    #[test]
    fn render_json_with_no_items_is_valid_empty_bundle() {
        let json = render_json(&[]);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        let files = value
            .pointer("/files")
            .and_then(serde_json::Value::as_array)
            .expect("files array");
        assert!(files.is_empty());
    }

    #[test]
    fn render_lines_headers_number_bodies_and_separate_files() {
        let items = vec![
            Stripped {
                path: PathBuf::from("src/lib.rs"),
                lang_name: "rust",
                original: String::new(),
                content: String::new(),
                numbered: "1: pub fn greet() …\n".to_owned(),
            },
            Stripped {
                path: PathBuf::from("src/main.rs"),
                lang_name: "rust",
                original: String::new(),
                content: String::new(),
                numbered: "1: fn main() …\n".to_owned(),
            },
        ];
        let out = render_lines(&items);
        assert_eq!(
            out,
            "src/lib.rs\n1: pub fn greet() …\n\nsrc/main.rs\n1: fn main() …\n"
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
