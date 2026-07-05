//! Input resolution: classify each path argument (file, folder, glob, or
//! `@name`) and unfold it to a deduped, sorted set of source files for the
//! strip pipeline.
//!
//! Each path argument carries an [`IgnoreMode`] that controls `.gitignore`
//! filtering and the CLI group's strip selection (`Option<StripSet>`; `None`
//! means no explicit `--strip`, fall through to config layering). Folders are
//! walked gitignore-aware; globs are expanded internally; query files
//! (`*.cty.{yaml,yml}`) unfold to their selected source files, contributing a
//! [`FileStrip`] that pairs the group's CLI strip with the query's own strip.
//! An argument of the form `@name` names a saved query: it resolves to
//! `<project>/.contasty/queries/<name>.cty.yaml` (then `.cty.yml`), else
//! `<global>/queries/<name>.cty.yaml` (then `.yml`), first hit wins, and then
//! unfolds exactly like a query file passed by path.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use globset::GlobBuilder;
use ignore::WalkBuilder;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::AppError;
use crate::config::{FileStrip, StripSet};

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

/// Resolve raw path arguments to a deduped, sorted set of source files paired
/// with per-file strip inputs ([`FileStrip`]).
///
/// When the same file appears in multiple groups, the last group's strip wins
/// (find-style last-wins dedup). `global_dir` is the resolved XDG global
/// contasty directory (`None` when unset), consulted only for `@name`
/// saved-query lookups.
///
/// # Errors
///
/// [`AppError::Input`] when a named path does not exist, a glob is malformed,
/// or a saved `@name` query is not found. [`AppError::Walk`] from the walker.
/// [`AppError::Query`] from query file resolution.
pub fn resolve(
    args: &[(PathBuf, IgnoreMode, Option<StripSet>)],
    cwd: &Path,
    global_dir: Option<&Path>,
) -> Result<Vec<(PathBuf, FileStrip)>, AppError> {
    let default = [(PathBuf::from("."), IgnoreMode::Enable, None)];
    let args = if args.is_empty() { &default[..] } else { args };
    let mut resolver = Resolver::new(cwd, global_dir);
    for (arg, mode, cli) in args {
        resolver.resolve_one(arg, *mode, *cli)?;
    }
    Ok(resolver.out.into_iter().collect())
}

/// Accumulating context for a single `resolve` call: the working directory,
/// the XDG global directory (for `@name` lookups), the query-file cycle
/// guard, and the deduped output map.
struct Resolver<'cwd> {
    cwd: &'cwd Path,
    global_dir: Option<PathBuf>,
    visited: BTreeSet<PathBuf>,
    out: BTreeMap<PathBuf, FileStrip>,
}

impl<'cwd> Resolver<'cwd> {
    fn new(cwd: &'cwd Path, global_dir: Option<&Path>) -> Self {
        Self {
            cwd,
            global_dir: global_dir.map(Path::to_path_buf),
            visited: BTreeSet::new(),
            out: BTreeMap::new(),
        }
    }

    fn resolve_one(
        &mut self,
        arg: &Path,
        mode: IgnoreMode,
        cli: Option<StripSet>,
    ) -> Result<(), AppError> {
        if let Some(name) = named_query(arg) {
            let resolved = self.resolve_named_query(name)?;
            return self.add_or_unfold(&resolved, mode, cli);
        }
        if is_glob(arg) {
            self.expand_glob(arg, mode, cli)
        } else if arg.is_dir() {
            self.walk_dir(arg, mode, cli)
        } else if arg.is_file() {
            self.add_or_unfold(arg, mode, cli)
        } else {
            Err(AppError::Input(format!(
                "path not found: {}",
                arg.display()
            )))
        }
    }

    /// Resolve `@name` to a saved query file: project queries first
    /// (`.cty.yaml` then `.cty.yml`, under `<cwd>/.contasty/queries/`), then
    /// the global queries dir in the same order. First hit wins.
    fn resolve_named_query(&self, name: &str) -> Result<PathBuf, AppError> {
        let mut candidates = vec![
            self.cwd
                .join(".contasty/queries")
                .join(format!("{name}.cty.yaml")),
            self.cwd
                .join(".contasty/queries")
                .join(format!("{name}.cty.yml")),
        ];
        if let Some(global) = &self.global_dir {
            candidates.push(global.join("queries").join(format!("{name}.cty.yaml")));
            candidates.push(global.join("queries").join(format!("{name}.cty.yml")));
        }
        candidates
            .iter()
            .find(|path| path.is_file())
            .cloned()
            .ok_or_else(|| {
                let searched = candidates
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                AppError::Input(format!(
                    "saved query `@{name}` not found; searched: {searched}"
                ))
            })
    }

    /// Add a regular file or unfold a query file into the output set.
    fn add_or_unfold(
        &mut self,
        path: &Path,
        mode: IgnoreMode,
        cli: Option<StripSet>,
    ) -> Result<(), AppError> {
        if !file_passes_mode(path, mode, self.cwd)? {
            return Ok(());
        }
        if is_query_file(path) {
            let (files, query_strip) =
                crate::query::resolve_query(path, mode, self.cwd, &mut self.visited)?;
            for file in files {
                self.out.insert(file, FileStrip::new(cli, query_strip));
            }
            return Ok(());
        }
        self.out
            .insert(normalize(path), FileStrip::new(cli, StripSet::empty()));
        Ok(())
    }

    /// Walk a directory with mode-appropriate gitignore filtering.
    fn walk_dir(
        &mut self,
        root: &Path,
        mode: IgnoreMode,
        cli: Option<StripSet>,
    ) -> Result<(), AppError> {
        match mode {
            IgnoreMode::Enable => self.walk_enable(root, cli),
            IgnoreMode::Disable => self.walk_disable(root, cli),
            IgnoreMode::Reverse => self.walk_reverse(root, cli),
        }
    }

    /// Standard gitignore-aware walk (default behaviour).
    fn walk_enable(&mut self, root: &Path, cli: Option<StripSet>) -> Result<(), AppError> {
        for entry in WalkBuilder::new(root).build() {
            let entry = entry?;
            if entry.file_type().is_some_and(|kind| kind.is_file()) {
                self.add_or_unfold(entry.path(), IgnoreMode::Enable, cli)?;
            }
        }
        Ok(())
    }

    /// Walk with gitignore filters off, still skipping `.git/` directories.
    fn walk_disable(&mut self, root: &Path, cli: Option<StripSet>) -> Result<(), AppError> {
        let walker = WalkBuilder::new(root)
            .standard_filters(false)
            .filter_entry(|entry| {
                !entry.file_type().is_some_and(|kind| kind.is_dir()) || entry.file_name() != ".git"
            })
            .build();
        for entry in walker {
            let entry = entry?;
            if entry.file_type().is_some_and(|kind| kind.is_file()) {
                self.add_or_unfold(entry.path(), IgnoreMode::Disable, cli)?;
            }
        }
        Ok(())
    }

    /// Walk with all filters off, keeping only paths the `.gitignore` marks
    /// ignored.
    fn walk_reverse(&mut self, root: &Path, cli: Option<StripSet>) -> Result<(), AppError> {
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
            if is_query_file(path) {
                let (files, query_strip) = crate::query::resolve_query(
                    path,
                    IgnoreMode::Reverse,
                    self.cwd,
                    &mut self.visited,
                )?;
                for file in files {
                    self.out.insert(file, FileStrip::new(cli, query_strip));
                }
            } else {
                self.out
                    .insert(normalize(path), FileStrip::new(cli, StripSet::empty()));
            }
        }
        Ok(())
    }

    /// Expand a glob: walk its literal prefix with mode-appropriate filters and
    /// match each entry's normalized path. Files contribute themselves, matched
    /// directories are walked. Zero matches warn to stderr and continue.
    fn expand_glob(
        &mut self,
        pattern: &Path,
        mode: IgnoreMode,
        cli: Option<StripSet>,
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
                self.walk_dir(entry.path(), mode, cli)?;
            } else if kind.is_some_and(|item| item.is_file()) {
                self.add_or_unfold(entry.path(), mode, cli)?;
            }
        }
        if hits == 0 {
            warn_no_match(glob_text);
        }
        Ok(())
    }
}

/// A saved-query reference: the whole argument is `@` plus the query name.
fn named_query(arg: &Path) -> Option<&str> {
    arg.to_str()?.strip_prefix('@')
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

/// Longest leading path of `pattern` with no glob metacharacter.
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
