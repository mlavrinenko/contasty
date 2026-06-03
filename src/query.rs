//! Query file (`*.cty.yaml` / `*.cty.yml`) parser and resolver.
//!
//! A query file is a saved, reusable selector that unfolds to a source-file
//! set. Selection is expressed in `.gitignore` syntax (bare line = include,
//! `!` = exclude) and mapped onto `ignore::gitignore::Gitignore` with
//! inverted semantics: gitignore "ignore" becomes "select", gitignore
//! "unignore" becomes "deselect".

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::Deserialize;

use crate::AppError;
use crate::inputs::{is_query_file, normalize};

/// Parsed query file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryFile {
    #[serde(default)]
    rules: Option<Rules>,
    #[serde(default)]
    import: Vec<ImportEntry>,
}

/// Selection patterns: inline string, list of strings, or external file.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Rules {
    Inline(String),
    List(Vec<String>),
    File { path: PathBuf },
}

/// An import entry: bare path string or `{ path, required }`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ImportEntry {
    Path(String),
    WithOptions {
        path: PathBuf,
        #[serde(default = "default_required")]
        required: bool,
    },
}

const fn default_required() -> bool {
    true
}

/// Resolve a query file to a set of source-file paths.
///
/// Parses the YAML, builds a gitignore matcher from its `rules`, walks
/// candidates `.gitignore`-aware, filters through the matcher (with
/// parent-directory checking), then recurses into `import` entries. Results
/// are unioned and deduped.
///
/// # Errors
///
/// Broken YAML, unknown field, missing required import, path escaping the
/// CWD, or a pattern compilation failure.
pub fn resolve_query(
    query_path: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, AppError> {
    let abs_query = normalize(&make_absolute(query_path, cwd));
    if !visited.insert(abs_query.clone()) {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&abs_query).map_err(|err| {
        AppError::Query(format!(
            "cannot read query file `{}`: {err}",
            abs_query.display()
        ))
    })?;
    let parsed: QueryFile = serde_yaml::from_str(&content).map_err(|err| {
        AppError::Query(format!("bad query file `{}`: {err}", abs_query.display()))
    })?;
    let query_dir = abs_query
        .parent()
        .map_or_else(|| cwd.to_path_buf(), Path::to_path_buf);
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();
    if let Some(rules) = parsed.rules {
        let selected = apply_rules(&rules, &query_dir, cwd, visited)?;
        out.extend(selected);
    }
    for entry in &parsed.import {
        let imported = resolve_import(entry, &query_dir, cwd, visited)?;
        out.extend(imported);
    }
    Ok(out.into_iter().collect())
}

/// Build a gitignore matcher from `rules`, walk candidates, and filter.
///
/// Uses `Gitignore::matched_path_or_any_parents` so a directory pattern like
/// `src` selects every file under `src/`. Semantics are inverted from
/// gitignore: a gitignore "ignore" match means "select", a "whitelist"
/// (negation) match means "deselect", and no match means "not selected". A
/// selected `*.cty.yaml` is itself a query file: it unfolds recursively (like
/// an `import`) rather than being emitted as content; the shared `visited` set
/// guards against cycles.
fn apply_rules(
    rules: &Rules,
    query_dir: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, AppError> {
    let (patterns, root) = load_patterns(rules, query_dir, cwd)?;
    let matcher = build_matcher(&patterns, &root)?;
    let mut out = Vec::new();
    if matcher.is_empty() {
        return Ok(out);
    }
    for entry in WalkBuilder::new(&root).build() {
        let entry = entry?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path();
        if !matcher.matched_path_or_any_parents(path, false).is_ignore() {
            continue;
        }
        if is_query_file(path) {
            out.extend(resolve_query(path, cwd, visited)?);
        } else {
            out.push(normalize(path));
        }
    }
    Ok(out)
}

/// Compile gitignore-syntax `patterns` into a matcher rooted at `root`.
fn build_matcher(patterns: &[String], root: &Path) -> Result<Gitignore, AppError> {
    let mut builder = GitignoreBuilder::new(root);
    for pat in patterns {
        builder
            .add_line(None, pat)
            .map_err(|err| AppError::Query(format!("bad pattern `{pat}`: {err}")))?;
    }
    builder
        .build()
        .map_err(|err| AppError::Query(format!("failed to build matcher: {err}")))
}

/// Extract pattern lines and the root directory they are relative to.
fn load_patterns(
    rules: &Rules,
    query_dir: &Path,
    cwd: &Path,
) -> Result<(Vec<String>, PathBuf), AppError> {
    match rules {
        Rules::Inline(text) => {
            let lines = parse_lines(text);
            Ok((lines, query_dir.to_path_buf()))
        }
        Rules::List(items) => Ok((items.clone(), query_dir.to_path_buf())),
        Rules::File { path } => {
            let abs = resolve_relative(path, query_dir);
            check_within_cwd(&abs, cwd)?;
            let content = fs::read_to_string(&abs).map_err(|err| {
                AppError::Query(format!("cannot read rules file `{}`: {err}", abs.display()))
            })?;
            let root = abs
                .parent()
                .map_or_else(|| query_dir.to_path_buf(), Path::to_path_buf);
            let lines = parse_lines(&content);
            Ok((lines, root))
        }
    }
}

/// Resolve an import entry to its file set.
fn resolve_import(
    entry: &ImportEntry,
    query_dir: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, AppError> {
    let (rel_path, required) = match entry {
        ImportEntry::Path(text) => (PathBuf::from(text), true),
        ImportEntry::WithOptions { path, required } => (path.clone(), *required),
    };
    let abs = resolve_relative(&rel_path, query_dir);
    check_within_cwd(&abs, cwd)?;
    if !abs.exists() {
        if required {
            return Err(AppError::Query(format!(
                "required import not found: `{}`",
                abs.display()
            )));
        }
        return Ok(Vec::new());
    }
    resolve_query(&abs, cwd, visited)
}

/// Split a multiline string into non-empty, non-comment lines.
fn parse_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Make a path absolute relative to `cwd` if it is not already.
fn make_absolute(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

/// Resolve `rel` against `base` and normalize.
fn resolve_relative(rel: &Path, base: &Path) -> PathBuf {
    normalize(&base.join(rel))
}

/// Lexical check: `abs_path` must stay within `cwd`. `../` that escapes is
/// rejected.
fn check_within_cwd(abs_path: &Path, cwd: &Path) -> Result<(), AppError> {
    let normalized = normalize(abs_path);
    let norm_cwd = normalize(cwd);
    if !normalized.starts_with(&norm_cwd) {
        return Err(AppError::Query(format!(
            "path escapes working directory: `{}` (cwd: `{}`)",
            normalized.display(),
            norm_cwd.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(&path, body).expect("write");
        path
    }

    #[test]
    fn query_inline_rules_unfolds_matching_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/a.rs", "fn a() {}\n");
        write(tmp.path(), "src/b.rs", "fn b() {}\n");
        write(tmp.path(), "lib/c.rs", "fn c() {}\n");
        let query = write(tmp.path(), "api.cty.yaml", "rules: |\n  src\n");
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 2, "{files:?}");
        assert!(
            files
                .iter()
                .all(|path| path.to_str().is_some_and(|text| text.contains("src")))
        );
    }

    #[test]
    fn query_negation_excludes_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/keep.rs", "fn k() {}\n");
        write(tmp.path(), "src/drop.rs", "fn d() {}\n");
        let body = "rules: |\n  src\n  !src/drop.rs\n";
        let query = write(tmp.path(), "q.cty.yaml", body);
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 1, "{files:?}");
        assert!(
            files
                .first()
                .expect("one")
                .to_str()
                .is_some_and(|t| t.contains("keep.rs"))
        );
    }

    #[test]
    fn query_list_form_rules() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/a.rs", "fn a() {}\n");
        write(tmp.path(), "src/b_test.rs", "fn b() {}\n");
        let body = "rules:\n  - \"src/**/*.rs\"\n  - \"!**/*_test.rs\"\n";
        let query = write(tmp.path(), "q.cty.yaml", body);
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 1, "{files:?}");
        assert!(
            files
                .first()
                .expect("one")
                .to_str()
                .is_some_and(|t| t.ends_with("a.rs"))
        );
    }

    #[test]
    fn query_external_rules_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/a.rs", "fn a() {}\n");
        write(tmp.path(), "src/b.rs", "fn b() {}\n");
        write(tmp.path(), "special.ignore", "src/a.rs\n");
        let body = "rules:\n  path: ./special.ignore\n";
        let query = write(tmp.path(), "q.cty.yaml", body);
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 1, "{files:?}");
        assert!(
            files
                .first()
                .expect("one")
                .to_str()
                .is_some_and(|t| t.ends_with("a.rs"))
        );
    }

    #[test]
    fn query_import_unions_results() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/a.rs", "fn a() {}\n");
        write(tmp.path(), "lib/b.rs", "fn b() {}\n");
        write(tmp.path(), "shared.cty.yaml", "rules: |\n  lib\n");
        let body = "rules: |\n  src\nimport:\n  - shared.cty.yaml\n";
        let query = write(tmp.path(), "main.cty.yaml", body);
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 2, "{files:?}");
    }

    #[test]
    fn query_missing_required_import_errors() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let body = "import:\n  - missing.cty.yaml\n";
        let query = write(tmp.path(), "q.cty.yaml", body);
        let mut visited = BTreeSet::new();
        let err = resolve_query(&query, tmp.path(), &mut visited)
            .expect_err("missing required import must error");
        assert!(matches!(err, AppError::Query(_)), "{err:?}");
    }

    #[test]
    fn query_optional_import_skips_silently() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/a.rs", "fn a() {}\n");
        let body = "rules: |\n  src\nimport:\n  - path: missing.cty.yaml\n    required: false\n";
        let query = write(tmp.path(), "q.cty.yaml", body);
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 1, "{files:?}");
    }

    #[test]
    fn query_cycle_guard_prevents_infinite_recursion() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/a.rs", "fn a() {}\n");
        write(
            tmp.path(),
            "a.cty.yaml",
            "rules: |\n  src\nimport:\n  - b.cty.yaml\n",
        );
        write(tmp.path(), "b.cty.yaml", "import:\n  - a.cty.yaml\n");
        let query = tmp.path().join("a.cty.yaml");
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 1, "{files:?}");
    }

    #[test]
    fn query_rules_matched_query_file_unfolds() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "real.rs", "fn r() {}\n");
        write(tmp.path(), "sub.cty.yaml", "rules: |\n  real.rs\n");
        // main selects only the sub-query file; its files arrive via unfold,
        // not by being emitted as YAML content.
        let query = write(tmp.path(), "main.cty.yaml", "rules: |\n  sub.cty.yaml\n");
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 1, "{files:?}");
        assert!(
            files
                .first()
                .expect("one")
                .to_str()
                .is_some_and(|t| t.ends_with("real.rs"))
        );
    }

    #[test]
    fn query_rules_cycle_guard_holds() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "real.rs", "fn r() {}\n");
        write(
            tmp.path(),
            "a.cty.yaml",
            "rules: |\n  real.rs\n  b.cty.yaml\n",
        );
        write(tmp.path(), "b.cty.yaml", "rules: |\n  a.cty.yaml\n");
        let query = tmp.path().join("a.cty.yaml");
        let mut visited = BTreeSet::new();
        let files = resolve_query(&query, tmp.path(), &mut visited).expect("resolve");
        assert_eq!(files.len(), 1, "{files:?}");
    }

    #[test]
    fn query_path_escape_errors() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let body = "import:\n  - ../../etc/something.cty.yaml\n";
        let query = write(tmp.path(), "q.cty.yaml", body);
        let mut visited = BTreeSet::new();
        let err = resolve_query(&query, tmp.path(), &mut visited).expect_err("escape must error");
        assert!(matches!(err, AppError::Query(_)), "{err:?}");
    }

    #[test]
    fn query_broken_yaml_errors() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = write(tmp.path(), "q.cty.yaml", "not: [valid: yaml");
        let mut visited = BTreeSet::new();
        let err =
            resolve_query(&query, tmp.path(), &mut visited).expect_err("broken yaml must error");
        assert!(matches!(err, AppError::Query(_)), "{err:?}");
    }

    #[test]
    fn query_unknown_field_errors() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = write(tmp.path(), "q.cty.yaml", "rules: |\n  src\nbogus: true\n");
        let mut visited = BTreeSet::new();
        let err =
            resolve_query(&query, tmp.path(), &mut visited).expect_err("unknown field must error");
        assert!(matches!(err, AppError::Query(_)), "{err:?}");
    }
}
