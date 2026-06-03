//! Strip pipeline over a resolved set of source files.
//!
//! Walking and glob expansion live in [`crate::inputs`]; this module takes the
//! already-resolved file set and dispatches each file to the language registry.

use std::fs;
use std::path::{Path, PathBuf};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::AppError;
use crate::config::{CategorySelection, Config};
use crate::lang::Registry;

/// One file's stripped representation, ready for rendering.
pub struct Stripped {
    /// Path to the source file (as the resolver reported it).
    pub path: PathBuf,
    /// Markdown fence info-string for this file's language (e.g. `"rust"`).
    pub lang_name: &'static str,
    /// Original source text before stripping.
    pub original: String,
    /// Stripped source text — declarations kept, bodies replaced with `ELISION`.
    pub content: String,
}

/// Strip every supported file in `files` (a resolved, deduped set from
/// [`crate::resolve`]).
///
/// Unsupported extensions and reserved query files (`*.cty.{yaml,yml}`) are
/// silently skipped. Output is sorted by path for deterministic rendering.
///
/// Category inclusion is resolved per language by layering built-in defaults,
/// config cross-language defaults (`[include]`), per-language config
/// (`[languages.<lang>.include]`), and the `cli` override (global, applied last).
///
/// # Errors
///
/// - [`AppError::Io`] from reading source files.
/// - [`AppError::CustomLang`] when a configured dynamic grammar fails to load.
/// - [`AppError::Rule`] / [`AppError::RuleParse`] / [`AppError::ParseFailed`]
///   when a language module misbehaves on a real file.
pub fn collect(
    files: &[PathBuf],
    cli: CategorySelection,
    config: &Config,
) -> Result<Vec<Stripped>, AppError> {
    let registry = Registry::with_config(config)?;
    // Parse + strip in parallel — tree-sitter parsing and any reformat pass
    // dominate the runtime and are per-file independent. `Registry` is `Sync`,
    // so one instance is shared read-only.
    let mut out: Vec<Stripped> = files
        .par_iter()
        .filter_map(|path| strip_one(path, &registry, cli, config).transpose())
        .collect::<Result<Vec<Stripped>, AppError>>()?;
    out.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(out)
}

fn strip_one(
    path: &Path,
    registry: &Registry,
    cli: CategorySelection,
    config: &Config,
) -> Result<Option<Stripped>, AppError> {
    // Reserve the query sub-extension: a `*.cty.yaml` would otherwise be detected
    // as YAML source and stripped instead of (later) unfolded.
    if crate::inputs::is_query_file(path) {
        return Ok(None);
    }
    let Some(language) = registry.detect(path) else {
        return Ok(None);
    };
    let drops = config.resolve_selection(language.name, cli);
    let source = fs::read_to_string(path)?;
    let content = language.strip(
        &source,
        path,
        drops.drop_tests,
        drops.drop_comments,
        drops.drop_imports,
        &config.compact,
    )?;
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

    use super::*;

    fn sel(
        comments: Option<bool>,
        imports: Option<bool>,
        tests: Option<bool>,
    ) -> CategorySelection {
        CategorySelection {
            comments,
            imports,
            tests,
        }
    }

    fn write_file(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).expect("write");
        path
    }

    #[test]
    fn collect_strips_a_rust_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = write_file(
            tmp.path(),
            "a.rs",
            "fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n",
        );
        let items =
            collect(&[path], CategorySelection::default(), &Config::default()).expect("collect");
        assert_eq!(items.len(), 1);
        let item = items.first().expect("one item");
        assert_eq!(item.lang_name, "rust");
        assert!(item.content.contains("fn add(lhs: i32, rhs: i32) -> i32"));
        assert!(item.content.contains("{}"));
        assert!(!item.content.contains("lhs + rhs"));
    }

    #[test]
    fn collect_skips_unknown_extensions() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = write_file(tmp.path(), "readme.txt", "plain text");
        let items =
            collect(&[path], CategorySelection::default(), &Config::default()).expect("collect");
        assert!(items.is_empty());
    }

    #[test]
    fn collect_skips_query_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = write_file(tmp.path(), "api.cty.yaml", "rules: []\n");
        let items =
            collect(&[path], CategorySelection::default(), &Config::default()).expect("collect");
        assert!(items.is_empty(), "query file must not be stripped as yaml");
    }

    #[test]
    fn collect_returns_files_sorted_by_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut paths = Vec::new();
        for name in ["zzz.rs", "aaa.rs", "mmm.rs"] {
            paths.push(write_file(
                tmp.path(),
                name,
                "fn id(value: i32) -> i32 { value }\n",
            ));
        }
        let items =
            collect(&paths, CategorySelection::default(), &Config::default()).expect("collect");
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
        let path = write_file(tmp.path(), "a.rs", src);

        let default_items = collect(
            std::slice::from_ref(&path),
            CategorySelection::default(),
            &Config::default(),
        )
        .expect("default");
        assert!(
            !default_items
                .first()
                .expect("one")
                .content
                .contains("mod tests")
        );

        let with_tests = collect(
            std::slice::from_ref(&path),
            sel(None, None, Some(true)),
            &Config::default(),
        )
        .expect("with tests");
        assert!(
            with_tests
                .first()
                .expect("one")
                .content
                .contains("mod tests")
        );

        let no_tests = collect(
            std::slice::from_ref(&path),
            sel(None, None, Some(false)),
            &Config::default(),
        )
        .expect("no tests");
        let no_item = no_tests.first().expect("one");
        assert!(!no_item.content.contains("mod tests"));
        assert!(no_item.content.contains("pub fn add"));
    }

    #[test]
    fn collect_drops_comments_when_requested() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = "/// kept when included\n\
                   pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n\
                   // trailing note\n";
        let path = write_file(tmp.path(), "a.rs", src);

        let with_comments = collect(
            std::slice::from_ref(&path),
            sel(Some(true), None, None),
            &Config::default(),
        )
        .expect("with comments");
        let with_item = with_comments.first().expect("one");
        assert!(with_item.content.contains("/// kept when included"));
        assert!(with_item.content.contains("// trailing note"));

        let no_comments = collect(
            std::slice::from_ref(&path),
            sel(Some(false), None, None),
            &Config::default(),
        )
        .expect("no comments");
        let no_item = no_comments.first().expect("one");
        assert!(!no_item.content.contains("kept when included"));
        assert!(no_item.content.contains("pub fn add"));
    }

    #[test]
    fn collect_drops_imports_when_requested() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = "use std::collections::HashMap;\n\
                   pub fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n";
        let path = write_file(tmp.path(), "a.rs", src);

        let with_imports = collect(
            std::slice::from_ref(&path),
            CategorySelection::default(),
            &Config::default(),
        )
        .expect("with imports");
        assert!(
            with_imports
                .first()
                .expect("one")
                .content
                .contains("use std::collections::HashMap")
        );

        let no_imports = collect(
            std::slice::from_ref(&path),
            sel(None, Some(false), None),
            &Config::default(),
        )
        .expect("no imports");
        let no_item = no_imports.first().expect("one");
        assert!(!no_item.content.contains("use std::collections::HashMap"));
        assert!(no_item.content.contains("pub fn add"));
    }
}
