//! Strip pipeline over a resolved set of source files.
//!
//! Walking and glob expansion live in [`crate::inputs`]; this module takes the
//! already-resolved file set and dispatches each file to the language registry.

use std::fs;
use std::path::{Path, PathBuf};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::AppError;
use crate::config::{Config, FileStrip};
use crate::lang::Registry;

/// One file's stripped representation, ready for rendering.
pub struct Stripped {
    /// Path to the source file (as the resolver reported it).
    pub path: PathBuf,
    /// Language display name, also the Markdown fence info-string (e.g. `"rust"`).
    pub lang_name: &'static str,
    /// Original source text before stripping.
    pub original: String,
    /// Skeleton view: declarations kept, elided bodies replaced with `{}`.
    /// Feeds the `markdown` / `json` formats and `--stats`.
    pub content: String,
    /// Line-numbered view (`N: <line>`): the default `lines` format's body.
    pub numbered: String,
}

/// Strip every supported file in `files` (a resolved, deduped set from
/// [`crate::resolve`], paired with per-file strip categories).
///
/// Unsupported extensions and reserved query files (`*.cty.{yaml,yml}`) are
/// silently skipped. Output is sorted by path for deterministic rendering.
///
/// # Errors
///
/// - [`AppError::Io`] from reading source files.
/// - [`AppError::CustomLang`] when a configured dynamic grammar fails to load.
/// - [`AppError::Rule`] / [`AppError::RuleParse`] / [`AppError::ParseFailed`]
///   when a language module misbehaves on a real file.
pub fn collect(files: &[(PathBuf, FileStrip)], config: &Config) -> Result<Vec<Stripped>, AppError> {
    let registry = Registry::with_config(config)?;
    let mut out: Vec<Stripped> = files
        .par_iter()
        .filter_map(|(path, strip)| strip_one(path, *strip, &registry, config).transpose())
        .collect::<Result<Vec<Stripped>, AppError>>()?;
    out.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(out)
}

fn strip_one(
    path: &Path,
    strip: FileStrip,
    registry: &Registry,
    config: &Config,
) -> Result<Option<Stripped>, AppError> {
    if crate::inputs::is_query_file(path) {
        return Ok(None);
    }
    let Some(language) = registry.detect(path) else {
        return Ok(None);
    };
    let drops = config.resolve_drops(language.name, strip);
    let source = fs::read_to_string(path)?;
    let views = language.strip_views(
        &source,
        path,
        drops.drop_tests,
        drops.drop_comments,
        drops.drop_imports,
        drops.drop_bodies,
        &config.compact,
    )?;
    Ok(Some(Stripped {
        path: path.to_path_buf(),
        lang_name: language.name,
        original: source,
        content: views.skeleton,
        numbered: views.numbered,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::config::StripSet;

    fn write_file(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).expect("write");
        path
    }

    fn with_strip(paths: &[PathBuf]) -> Vec<(PathBuf, FileStrip)> {
        with_custom_strip(paths, StripSet::DEFAULT)
    }

    fn with_custom_strip(paths: &[PathBuf], strip: StripSet) -> Vec<(PathBuf, FileStrip)> {
        paths
            .iter()
            .map(|path| (path.clone(), FileStrip::new(Some(strip), StripSet::empty())))
            .collect()
    }

    #[test]
    fn collect_strips_a_rust_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = write_file(
            tmp.path(),
            "a.rs",
            "fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n",
        );
        let items = collect(&with_strip(&[path]), &Config::default()).expect("collect");
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
        let items = collect(&with_strip(&[path]), &Config::default()).expect("collect");
        assert!(items.is_empty());
    }

    #[test]
    fn collect_skips_query_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = write_file(tmp.path(), "api.cty.yaml", "rules: []\n");
        let items = collect(&with_strip(&[path]), &Config::default()).expect("collect");
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
        let items = collect(&with_strip(&paths), &Config::default()).expect("collect");
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

        let keep_tests = StripSet::empty().insert(StripSet::BODY);
        let default_items = collect(
            &with_custom_strip(std::slice::from_ref(&path), keep_tests),
            &Config::default(),
        )
        .expect("keep tests");
        assert!(
            default_items
                .first()
                .expect("one")
                .content
                .contains("mod tests")
        );

        let strip_tests = StripSet::empty()
            .insert(StripSet::TESTS)
            .insert(StripSet::BODY);
        let no_items = collect(
            &with_custom_strip(std::slice::from_ref(&path), strip_tests),
            &Config::default(),
        )
        .expect("strip tests");
        let no_item = no_items.first().expect("one");
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

        let keep_comments = StripSet::empty().insert(StripSet::BODY);
        let with_comments = collect(
            &with_custom_strip(std::slice::from_ref(&path), keep_comments),
            &Config::default(),
        )
        .expect("keep comments");
        let with_item = with_comments.first().expect("one");
        assert!(with_item.content.contains("/// kept when included"));
        assert!(with_item.content.contains("// trailing note"));

        let strip_comments = StripSet::empty()
            .insert(StripSet::COMMENTS)
            .insert(StripSet::BODY);
        let no_comments = collect(
            &with_custom_strip(std::slice::from_ref(&path), strip_comments),
            &Config::default(),
        )
        .expect("strip comments");
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

        let keep_imports = StripSet::empty().insert(StripSet::BODY);
        let with_imports = collect(
            &with_custom_strip(std::slice::from_ref(&path), keep_imports),
            &Config::default(),
        )
        .expect("keep imports");
        assert!(
            with_imports
                .first()
                .expect("one")
                .content
                .contains("use std::collections::HashMap")
        );

        let strip_imports = StripSet::empty()
            .insert(StripSet::IMPORTS)
            .insert(StripSet::BODY);
        let no_imports = collect(
            &with_custom_strip(std::slice::from_ref(&path), strip_imports),
            &Config::default(),
        )
        .expect("strip imports");
        let no_item = no_imports.first().expect("one");
        assert!(!no_item.content.contains("use std::collections::HashMap"));
        assert!(no_item.content.contains("pub fn add"));
    }

    #[test]
    fn collect_keeps_bodies_when_not_stripped() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = write_file(
            tmp.path(),
            "a.rs",
            "fn add(lhs: i32, rhs: i32) -> i32 { lhs + rhs }\n",
        );
        let keep_bodies = StripSet::empty();
        let items = collect(&with_custom_strip(&[path], keep_bodies), &Config::default())
            .expect("keep bodies");
        let item = items.first().expect("one");
        assert!(item.content.contains("lhs + rhs"), "body kept");
        assert!(!item.content.contains("{}"), "no elision marker");
    }
}
