//! Directory walker that respects `.gitignore` and dispatches to the language
//! registry.

use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::AppError;
use crate::config::CompactConfig;
use crate::lang::Registry;

/// One file's stripped representation, ready for rendering.
pub struct Stripped {
    /// Path to the source file (as the walker reported it).
    pub path: PathBuf,
    /// Markdown fence info-string for this file's language (e.g. `"rust"`).
    pub lang_name: &'static str,
    /// Original source text before stripping.
    pub original: String,
    /// Stripped source text — declarations kept, bodies replaced with `ELISION`.
    pub content: String,
}

/// Walk `root` (gitignore-aware) and strip every supported file.
///
/// `root` may be a file or a directory. Unsupported extensions are silently
/// skipped. Output is sorted by path for deterministic rendering.
///
/// When `drop_tests` is true, `#[cfg(test)]` modules and `#[test]` functions
/// are removed entirely instead of being shown with their signatures. When
/// `drop_comments` is true, every comment (including doc comments) is removed.
///
/// # Errors
///
/// - [`AppError::Io`] from reading source files.
/// - [`AppError::Walk`] from the `ignore` crate.
/// - [`AppError::LangLoad`] / [`AppError::Query`] / [`AppError::ParseFailed`]
///   when a language module misbehaves on a real file.
pub fn collect(
    root: &Path,
    drop_tests: bool,
    drop_comments: bool,
    compact: &CompactConfig,
) -> Result<Vec<Stripped>, AppError> {
    let registry = Registry::new()?;
    let mut out: Vec<Stripped> = Vec::new();
    for entry in WalkBuilder::new(root).build() {
        let entry = entry?;
        if !is_file(&entry) {
            continue;
        }
        if let Some(stripped) =
            strip_one(entry.path(), &registry, drop_tests, drop_comments, compact)?
        {
            out.push(stripped);
        }
    }
    out.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(out)
}

fn is_file(entry: &ignore::DirEntry) -> bool {
    entry.file_type().is_some_and(|ft| ft.is_file())
}

fn strip_one(
    path: &Path,
    registry: &Registry,
    drop_tests: bool,
    drop_comments: bool,
    compact: &CompactConfig,
) -> Result<Option<Stripped>, AppError> {
    let Some(language) = registry.detect(path) else {
        return Ok(None);
    };
    let source = fs::read_to_string(path)?;
    let content = language.strip(&source, path, drop_tests, drop_comments, compact)?;
    Ok(Some(Stripped {
        path: path.to_path_buf(),
        lang_name: language.name,
        original: source,
        content,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use super::*;

    #[test]
    fn collect_strips_rust_files_in_a_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("a.rs");
        let mut file = fs::File::create(&path).expect("create");
        writeln!(file, "fn add(lhs: i32, rhs: i32) -> i32 {{ lhs + rhs }}").expect("write");

        let items = collect(tmp.path(), false, false, &CompactConfig::default()).expect("collect");
        assert_eq!(items.len(), 1);
        let item = items.first().expect("one item");
        assert_eq!(item.lang_name, "rust");
        assert!(item.content.contains("fn add(lhs: i32, rhs: i32) -> i32"));
        assert!(item.content.contains("{/*CTY*/}"));
        assert!(!item.content.contains("lhs + rhs"));
    }

    #[test]
    fn collect_skips_unknown_extensions() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("readme.txt");
        fs::write(&path, "plain text").expect("write");

        let items = collect(tmp.path(), false, false, &CompactConfig::default()).expect("collect");
        assert!(items.is_empty());
    }

    #[test]
    fn collect_returns_files_sorted_by_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        for name in ["zzz.rs", "aaa.rs", "mmm.rs"] {
            fs::write(
                tmp.path().join(name),
                "fn id(value: i32) -> i32 { value }\n",
            )
            .expect("write");
        }

        let items = collect(tmp.path(), false, false, &CompactConfig::default()).expect("collect");
        let names: Vec<_> = items
            .iter()
            .map(|item| {
                item.path
                    .file_name()
                    .and_then(|os| os.to_str())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(names, vec!["aaa.rs", "mmm.rs", "zzz.rs"]);
    }

    #[test]
    fn collect_drops_tests_when_requested() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = "pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n\n\
                   #[cfg(test)]\n\
                   mod tests {\n    \
                       #[test]\n    \
                       fn it_works() { assert_eq!(add(1, 1), 2); }\n\
                   }\n";
        fs::write(tmp.path().join("a.rs"), src).expect("write");

        let with_tests =
            collect(tmp.path(), false, false, &CompactConfig::default()).expect("with tests");
        let with_item = with_tests.first().expect("one");
        assert!(with_item.content.contains("mod tests"));

        let no_tests =
            collect(tmp.path(), true, false, &CompactConfig::default()).expect("no tests");
        let no_item = no_tests.first().expect("one");
        assert!(!no_item.content.contains("mod tests"));
        assert!(!no_item.content.contains("cfg(test)"));
        assert!(no_item.content.contains("pub fn add"));
    }

    #[test]
    fn collect_drops_comments_when_requested() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = "/// kept by default\n\
                   pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n\
                   // trailing note\n";
        fs::write(tmp.path().join("a.rs"), src).expect("write");

        let with_comments =
            collect(tmp.path(), false, false, &CompactConfig::default()).expect("with comments");
        let with_item = with_comments.first().expect("one");
        assert!(with_item.content.contains("/// kept by default"));
        assert!(with_item.content.contains("// trailing note"));

        let no_comments =
            collect(tmp.path(), false, true, &CompactConfig::default()).expect("no comments");
        let no_item = no_comments.first().expect("one");
        assert!(!no_item.content.contains("kept by default"));
        assert!(!no_item.content.contains("trailing note"));
        assert!(no_item.content.contains("pub fn add"));
    }
}
