//! Query file (`*.cty.yaml` / `*.cty.yml`) parser and resolver.
//!
//! A query file is a saved, reusable selector that unfolds to a source-file
//! set. Selection is expressed in `.gitignore` syntax (bare line = include,
//! `!` = exclude) and mapped onto `ignore::gitignore::Gitignore` with
//! inverted semantics: gitignore "ignore" becomes "select", gitignore
//! "unignore" becomes "deselect".
//!
//! `rules` patterns are always rooted at `cwd` (the scanned project's working
//! directory), never at the query file's own directory — a saved query in
//! `.contasty/queries/` or the XDG global dir describes the *project*, not
//! its own location, so it selects correctly regardless of where it lives. An
//! external `{ path }` rule file is located relative to the query file's
//! directory (like an `import`), but the patterns it contains still root at
//! `cwd`. Only external rule files and `import` targets — trusted,
//! config-referenced machinery — may live outside `cwd`; the walker that
//! selects source files always roots at `cwd`, so a selected file is
//! guaranteed to live under it regardless of where the query/rule file does.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::Deserialize;

use crate::AppError;
use crate::config::{StripConfig, StripSet};
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
    #[serde(default)]
    strip: Option<StripConfig>,
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

/// Resolve a query file to a set of source-file paths and its strip set.
///
/// Parses the YAML, builds a gitignore matcher from its `rules`, walks
/// candidates with mode-appropriate gitignore filtering, filters through
/// the matcher (with parent-directory checking), then recurses into
/// `import` entries. Results are unioned and deduped.
///
/// Returns the resolved files and the query's own `strip` set (empty if
/// unset). The caller unions this with the CLI strip set.
///
/// The query's own `ignore` field (if set) overrides the ambient `mode`;
/// otherwise the ambient mode applies.
///
/// # Errors
///
/// Broken YAML, unknown field, missing required import, or a pattern
/// compilation failure.
pub fn resolve_query(
    query_path: &Path,
    mode: IgnoreMode,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<(Vec<PathBuf>, StripSet), AppError> {
    let abs_query = normalize(&make_absolute(query_path, cwd));
    if !visited.insert(abs_query.clone()) {
        return Ok((Vec::new(), StripSet::empty()));
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
    let query_strip = parsed.strip.map_or(StripSet::empty(), |sc| sc.0);
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
    Ok((out.into_iter().collect(), query_strip))
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
///
/// The matcher and the walker both root at `cwd`, not `query_dir` — see the
/// module docs for why.
fn apply_rules(
    rules: &Rules,
    mode: IgnoreMode,
    query_dir: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, AppError> {
    let patterns = load_patterns(rules, query_dir)?;
    let matcher = build_matcher(&patterns, cwd)?;
    let mut out = Vec::new();
    if matcher.is_empty() {
        return Ok(out);
    }
    let walker = build_rules_walker(cwd, mode);
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
            let (files, _) = resolve_query(path, mode, cwd, visited)?;
            out.extend(files);
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

/// Extract pattern lines from `rules`. An external file is located relative to
/// `query_dir` (like an `import`), but the returned lines are just raw
/// pattern text — the caller (`apply_rules`) roots them at `cwd`, not at the
/// file's own directory, regardless of which form produced them.
fn load_patterns(rules: &Rules, query_dir: &Path) -> Result<Vec<String>, AppError> {
    match rules {
        Rules::Inline(text) => Ok(parse_lines(text)),
        Rules::List(items) => Ok(items.clone()),
        Rules::File { path } => {
            let abs = resolve_relative(path, query_dir);
            let content = fs::read_to_string(&abs).map_err(|err| {
                AppError::Query(format!("cannot read rules file `{}`: {err}", abs.display()))
            })?;
            Ok(parse_lines(&content))
        }
    }
}

/// Resolve an import entry to its file set. The import target is located
/// relative to `query_dir` and may live outside `cwd` (trusted,
/// config-referenced machinery, like an external rules file) — the imported
/// query's own `rules` still root at `cwd`, so its selection stays confined
/// regardless of where the imported file itself sits on disk.
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
    if !abs.exists() {
        if required {
            return Err(AppError::Query(format!(
                "required import not found: `{}`",
                abs.display()
            )));
        }
        return Ok(Vec::new());
    }
    let (files, _) = resolve_query(&abs, mode, cwd, visited)?;
    Ok(files)
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

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
