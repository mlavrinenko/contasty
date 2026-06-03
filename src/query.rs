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
use crate::inputs::{IgnoreMode, is_query_file, normalize};

/// Parsed query file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryFile {
    #[serde(default)]
    ignore: Option<IgnoreMode>,
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
/// candidates with mode-appropriate gitignore filtering, filters through
/// the matcher (with parent-directory checking), then recurses into
/// `import` entries. Results are unioned and deduped.
///
/// The query's own `ignore` field (if set) overrides the ambient `mode`;
/// otherwise the ambient mode applies.
///
/// # Errors
///
/// Broken YAML, unknown field, missing required import, path escaping the
/// CWD, or a pattern compilation failure.
pub fn resolve_query(
    query_path: &Path,
    mode: IgnoreMode,
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
    let effective_mode = parsed.ignore.unwrap_or(mode);
    let query_dir = abs_query
        .parent()
        .map_or_else(|| cwd.to_path_buf(), Path::to_path_buf);
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();
    if let Some(rules) = parsed.rules {
        let selected = apply_rules(&rules, effective_mode, &query_dir, cwd, visited)?;
        out.extend(selected);
    }
    for entry in &parsed.import {
        let imported = resolve_import(entry, mode, &query_dir, cwd, visited)?;
        out.extend(imported);
    }
    Ok(out.into_iter().collect())
}

/// Build a gitignore matcher from `rules`, walk candidates with
/// mode-appropriate filtering, and filter.
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
    mode: IgnoreMode,
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
    let walker = build_rules_walker(&root, mode);
    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path();
        if !matcher.matched_path_or_any_parents(path, false).is_ignore() {
            continue;
        }
        if is_query_file(path) {
            out.extend(resolve_query(path, mode, cwd, visited)?);
        } else {
            out.push(normalize(path));
        }
    }
    Ok(out)
}

/// Build a walker for `apply_rules` under the given gitignore mode.
fn build_rules_walker(root: &Path, mode: IgnoreMode) -> ignore::Walk {
    match mode {
        IgnoreMode::Enable => WalkBuilder::new(root).build(),
        IgnoreMode::Disable | IgnoreMode::Reverse => WalkBuilder::new(root)
            .standard_filters(false)
            .filter_entry(|entry| {
                !entry.file_type().is_some_and(|kind| kind.is_dir()) || entry.file_name() != ".git"
            })
            .build(),
    }
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
    mode: IgnoreMode,
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
    resolve_query(&abs, mode, cwd, visited)
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
#[path = "query_tests.rs"]
mod tests;
