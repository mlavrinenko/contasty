//! Input resolution: classify each path argument (file, folder, or glob) and
//! unfold it to a deduped, sorted set of source files for the strip pipeline.
//!
//! Each path carries an [`IgnoreMode`] that controls `.gitignore` filtering:
//! `enable` (default, respect gitignore), `disable` (include ignored files),
//! or `reverse` (only ignored files). Folders are walked gitignore-aware;
//! globs are expanded internally; query files (`*.cty.{yaml,yml}`) unfold
//! to their selected source files.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use globset::GlobBuilder;
use ignore::WalkBuilder;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::AppError;

/// Glob metacharacters that mark an argument for expansion.
const GLOB_META: [char; 4] = ['*', '?', '[', '{'];

/// How `.gitignore` filtering applies to a path argument's candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum IgnoreMode {
    /// Respect `.gitignore` — only non-ignored files (default).
    #[default]
    Enable,
    /// Ignore `.gitignore` — include ignored files too (everything).
    Disable,
    /// Invert — only `.gitignore`d files.
    Reverse,
}

/// Resolve raw path arguments to a deduped, sorted set of source files.
///
/// Each argument pairs a path with an [`IgnoreMode`] controlling gitignore
/// filtering. Empty input defaults to the current directory with [`IgnoreMode::Enable`].
///
/// # Errors
///
/// [`AppError::Input`] when a named path does not exist or a glob is
/// malformed. [`AppError::Walk`] from the walker. [`AppError::Query`]
/// from query file resolution.
pub fn resolve(args: &[(PathBuf, IgnoreMode)], cwd: &Path) -> Result<Vec<PathBuf>, AppError> {
    let default = [(PathBuf::from("."), IgnoreMode::Enable)];
    let args = if args.is_empty() { &default[..] } else { args };
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();
    let mut visited: BTreeSet<PathBuf> = BTreeSet::new();
    for (arg, mode) in args {
        resolve_one(arg, *mode, cwd, &mut visited, &mut out)?;
    }
    Ok(out.into_iter().collect())
}

fn resolve_one(
    arg: &Path,
    mode: IgnoreMode,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    if is_glob(arg) {
        expand_glob(arg, mode, cwd, visited, out)
    } else if arg.is_dir() {
        walk_dir(arg, mode, cwd, visited, out)
    } else if arg.is_file() {
        add_or_unfold(arg, mode, cwd, visited, out)
    } else {
        Err(AppError::Input(format!(
            "path not found: {}",
            arg.display()
        )))
    }
}

/// True when any component carries a glob metacharacter.
fn is_glob(path: &Path) -> bool {
    path.to_str()
        .is_some_and(|text| text.chars().any(|ch| GLOB_META.contains(&ch)))
}

/// A query file: `*.cty.yaml` / `*.cty.yml`.
pub(crate) fn is_query_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".cty.yaml") || name.ends_with(".cty.yml"))
}

/// Add a regular file or unfold a query file into the output set.
///
/// In `Enable` mode, files pass unconditionally (the walker already filtered
/// ignored entries). In `Disable`, every named file is admitted. In `Reverse`,
/// a file must be `.gitignore`d to qualify.
fn add_or_unfold(
    path: &Path,
    mode: IgnoreMode,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    if !file_passes_mode(path, mode, cwd)? {
        return Ok(());
    }
    if is_query_file(path) {
        let files = crate::query::resolve_query(path, mode, cwd, visited)?;
        out.extend(files);
        return Ok(());
    }
    out.insert(normalize(path));
    Ok(())
}

/// Test a named file against the gitignore mode filter.
fn file_passes_mode(path: &Path, mode: IgnoreMode, cwd: &Path) -> Result<bool, AppError> {
    match mode {
        IgnoreMode::Enable | IgnoreMode::Disable => Ok(true),
        IgnoreMode::Reverse => {
            let abs_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            };
            let parent = abs_path.parent().unwrap_or(&abs_path);
            let matcher = build_gitignore_matcher(parent)?;
            Ok(is_ignored(&matcher, parent, &abs_path))
        }
    }
}

/// Walk a directory with mode-appropriate gitignore filtering.
fn walk_dir(
    root: &Path,
    mode: IgnoreMode,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    match mode {
        IgnoreMode::Enable => walk_enable(root, cwd, visited, out),
        IgnoreMode::Disable => walk_disable(root, cwd, visited, out),
        IgnoreMode::Reverse => walk_reverse(root, cwd, visited, out),
    }
}

/// Standard gitignore-aware walk (default behaviour).
fn walk_enable(
    root: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    for entry in WalkBuilder::new(root).build() {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_file()) {
            add_or_unfold(entry.path(), IgnoreMode::Enable, cwd, visited, out)?;
        }
    }
    Ok(())
}

/// Walk with gitignore filters off, still skipping `.git/` directories.
fn walk_disable(
    root: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    let walker = WalkBuilder::new(root)
        .standard_filters(false)
        .filter_entry(|entry| {
            !entry.file_type().is_some_and(|kind| kind.is_dir()) || entry.file_name() != ".git"
        })
        .build();
    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_file()) {
            add_or_unfold(entry.path(), IgnoreMode::Disable, cwd, visited, out)?;
        }
    }
    Ok(())
}

/// Walk with all filters off, keeping only paths the `.gitignore` marks
/// ignored. Skips `.git/` directories to avoid VCS internals.
fn walk_reverse(
    root: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    let matcher = build_gitignore_matcher(root)?;
    let walker = WalkBuilder::new(root)
        .standard_filters(false)
        .filter_entry(|entry| {
            !entry.file_type().is_some_and(|kind| kind.is_dir()) || entry.file_name() != ".git"
        })
        .build();
    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path();
        if !is_ignored(&matcher, root, path) {
            continue;
        }
        // Already passed the reverse filter; add or unfold directly.
        if is_query_file(path) {
            let files = crate::query::resolve_query(path, IgnoreMode::Reverse, cwd, visited)?;
            out.extend(files);
        } else {
            out.insert(normalize(path));
        }
    }
    Ok(())
}

/// Build a composite gitignore matcher from every `.gitignore` under `root`.
fn build_gitignore_matcher(root: &Path) -> Result<Gitignore, AppError> {
    let mut builder = GitignoreBuilder::new(root);
    let walker = WalkBuilder::new(root)
        .standard_filters(false)
        .filter_entry(|entry| {
            !entry.file_type().is_some_and(|kind| kind.is_dir()) || entry.file_name() != ".git"
        })
        .build();
    for entry in walker {
        let entry = entry?;
        if entry.file_name() == ".gitignore" && entry.file_type().is_some_and(|kind| kind.is_file())
        {
            builder.add(entry.path());
        }
    }
    builder
        .build()
        .map_err(|err| AppError::Input(format!("failed to build gitignore matcher: {err}")))
}

/// True when `path` is marked ignored by `matcher` (rooted at `root`).
fn is_ignored(matcher: &Gitignore, root: &Path, path: &Path) -> bool {
    if matcher.is_empty() {
        return false;
    }
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    matcher
        .matched_path_or_any_parents(relative, false)
        .is_ignore()
}

/// Expand a glob: walk its literal prefix with mode-appropriate filters and
/// match each entry's normalized path. Files contribute themselves, matched
/// directories are walked. Zero matches warn to stderr and continue.
fn expand_glob(
    pattern: &Path,
    mode: IgnoreMode,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    let Some(glob_text) = pattern.to_str() else {
        return Ok(());
    };
    let matcher = GlobBuilder::new(glob_text)
        .literal_separator(true)
        .build()
        .map_err(|err| AppError::Input(format!("bad glob `{glob_text}`: {err}")))?
        .compile_matcher();
    let prefix = literal_prefix(pattern);
    if !prefix.exists() {
        warn_no_match(glob_text);
        return Ok(());
    }
    let mut hits = 0_usize;
    let walker = build_glob_walker(&prefix, mode);
    for entry in walker {
        let entry = entry?;
        if !matcher.is_match(normalize(entry.path())) {
            continue;
        }
        hits += 1;
        let kind = entry.file_type();
        if kind.is_some_and(|item| item.is_dir()) {
            walk_dir(entry.path(), mode, cwd, visited, out)?;
        } else if kind.is_some_and(|item| item.is_file()) {
            add_or_unfold(entry.path(), mode, cwd, visited, out)?;
        }
    }
    if hits == 0 {
        warn_no_match(glob_text);
    }
    Ok(())
}

/// Build a walker for glob literal-prefix expansion under the given mode.
fn build_glob_walker(prefix: &Path, mode: IgnoreMode) -> ignore::Walk {
    match mode {
        IgnoreMode::Enable => WalkBuilder::new(prefix).build(),
        IgnoreMode::Disable | IgnoreMode::Reverse => WalkBuilder::new(prefix)
            .standard_filters(false)
            .filter_entry(|entry| {
                !entry.file_type().is_some_and(|kind| kind.is_dir()) || entry.file_name() != ".git"
            })
            .build(),
    }
}

fn warn_no_match(glob_text: &str) {
    eprintln!("contasty: warning: glob matched no files: {glob_text}");
}

/// Longest leading path of `pattern` with no glob metacharacter — the
/// directory to root the expansion walk at. Defaults to `.` when the first
/// component is a glob.
fn literal_prefix(pattern: &Path) -> PathBuf {
    let mut prefix = PathBuf::new();
    for comp in pattern.components() {
        let part = comp.as_os_str();
        if part
            .to_str()
            .is_some_and(|text| text.chars().any(|ch| GLOB_META.contains(&ch)))
        {
            break;
        }
        prefix.push(part);
    }
    if prefix.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        prefix
    }
}

/// Lexically normalize a path: drop `.`, resolve `..`, no filesystem access.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

#[cfg(test)]
#[path = "inputs_tests.rs"]
mod tests;
