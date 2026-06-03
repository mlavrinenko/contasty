//! Input resolution: classify each path argument (file, folder, or glob) and
//! unfold it to a deduped, sorted set of source files for the strip pipeline.
//!
//! Folders are walked `.gitignore`-aware; globs are expanded against the
//! filesystem (file matches contribute themselves, directory matches are
//! walked); query files (`*.cty.{yaml,yml}`) are unfolded to their selected
//! source files. Paths are lexically normalized so a file reached by several
//! arguments appears once.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use globset::GlobBuilder;
use ignore::WalkBuilder;

use crate::AppError;

/// Glob metacharacters that mark an argument for expansion.
const GLOB_META: [char; 4] = ['*', '?', '[', '{'];

/// Resolve raw path arguments to a deduped, sorted set of source files.
///
/// Each argument is a source file, a folder (walked `.gitignore`-aware), a
/// glob, or a query file (`*.cty.{yaml,yml}` — unfolded to its selected
/// files). Empty input defaults to the current directory. `cwd` is the
/// working directory used to sandbox query-derived paths.
///
/// # Errors
///
/// [`AppError::Input`] when a named (non-glob) path does not exist or a glob
/// is malformed. [`AppError::Walk`] from the `ignore` walker.
/// [`AppError::Query`] from query file resolution.
pub fn resolve(args: &[PathBuf], cwd: &Path) -> Result<Vec<PathBuf>, AppError> {
    let default = [PathBuf::from(".")];
    let args = if args.is_empty() { &default[..] } else { args };
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();
    let mut visited: BTreeSet<PathBuf> = BTreeSet::new();
    for arg in args {
        resolve_one(arg, cwd, &mut visited, &mut out)?;
    }
    Ok(out.into_iter().collect())
}

fn resolve_one(
    arg: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    if is_glob(arg) {
        expand_glob(arg, cwd, visited, out)
    } else if arg.is_dir() {
        walk_dir(arg, cwd, visited, out)
    } else if arg.is_file() {
        add_or_unfold(arg, cwd, visited, out)
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
fn add_or_unfold(
    path: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    if is_query_file(path) {
        let files = crate::query::resolve_query(path, cwd, visited)?;
        out.extend(files);
        return Ok(());
    }
    out.insert(normalize(path));
    Ok(())
}

/// Walk a directory `.gitignore`-aware, adding every file it yields.
fn walk_dir(
    root: &Path,
    cwd: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut BTreeSet<PathBuf>,
) -> Result<(), AppError> {
    for entry in WalkBuilder::new(root).build() {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_file()) {
            add_or_unfold(entry.path(), cwd, visited, out)?;
        }
    }
    Ok(())
}

/// Expand a glob: walk its literal prefix `.gitignore`-aware and match each
/// entry's normalized path; files contribute themselves, matched directories
/// are walked. Zero matches warn to stderr and continue.
fn expand_glob(
    pattern: &Path,
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
    for entry in WalkBuilder::new(&prefix).build() {
        let entry = entry?;
        if !matcher.is_match(normalize(entry.path())) {
            continue;
        }
        hits += 1;
        let kind = entry.file_type();
        if kind.is_some_and(|item| item.is_dir()) {
            walk_dir(entry.path(), cwd, visited, out)?;
        } else if kind.is_some_and(|item| item.is_file()) {
            add_or_unfold(entry.path(), cwd, visited, out)?;
        }
    }
    if hits == 0 {
        warn_no_match(glob_text);
    }
    Ok(())
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
    fn resolve_unions_and_dedups_file_and_folder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let one = write(tmp.path(), "a.rs", "fn a() {}\n");
        write(tmp.path(), "b.rs", "fn b() {}\n");
        let files = resolve(&[tmp.path().to_path_buf(), one], tmp.path()).expect("resolve");
        assert_eq!(files.len(), 2, "{files:?}");
    }

    #[test]
    fn resolve_missing_path_errors() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("nope.rs");
        let err = resolve(&[missing], tmp.path()).expect_err("missing must error");
        assert!(matches!(err, AppError::Input(_)), "{err:?}");
    }

    #[test]
    fn resolve_glob_matches_files_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "a.rs", "fn a() {}\n");
        write(tmp.path(), "b.rs", "fn b() {}\n");
        write(tmp.path(), "c.txt", "text\n");
        let files = resolve(&[tmp.path().join("*.rs")], tmp.path()).expect("resolve");
        assert_eq!(files.len(), 2, "{files:?}");
        assert!(
            files
                .iter()
                .all(|path| path.extension().is_some_and(|ext| ext == "rs"))
        );
    }

    #[test]
    fn resolve_glob_walks_matched_directory_subtree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "crates/x/src/lib.rs", "fn x() {}\n");
        write(tmp.path(), "crates/y/src/lib.rs", "fn y() {}\n");
        write(tmp.path(), "crates/x/readme.md", "doc\n");
        let files = resolve(&[tmp.path().join("crates/*/src")], tmp.path()).expect("resolve");
        assert_eq!(files.len(), 2, "{files:?}");
        assert!(files.iter().all(|path| path.ends_with("src/lib.rs")));
    }

    #[test]
    fn resolve_unfolds_query_file_in_folder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/a.rs", "fn a() {}\n");
        write(tmp.path(), "lib/b.rs", "fn b() {}\n");
        write(tmp.path(), "api.cty.yaml", "rules: |\n  src\n");
        let files = resolve(&[tmp.path().to_path_buf()], tmp.path()).expect("resolve");
        assert_eq!(
            files.len(),
            2,
            "walk adds lib/b.rs, query adds src/a.rs: {files:?}"
        );
        assert!(
            files
                .iter()
                .any(|path| path.to_str().is_some_and(|text| text.contains("src/a.rs")))
        );
    }

    #[test]
    fn resolve_unfolds_query_file_as_direct_arg() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "src/a.rs", "fn a() {}\n");
        write(tmp.path(), "lib/b.rs", "fn b() {}\n");
        let query = write(tmp.path(), "api.cty.yaml", "rules: |\n  src\n");
        let files = resolve(&[query], tmp.path()).expect("resolve");
        assert_eq!(files.len(), 1, "{files:?}");
    }

    #[test]
    fn resolve_glob_zero_match_is_not_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "a.txt", "text\n");
        let files = resolve(&[tmp.path().join("*.rs")], tmp.path()).expect("zero match warns");
        assert!(files.is_empty(), "{files:?}");
    }

    #[test]
    fn normalize_resolves_dot_and_parent() {
        assert_eq!(
            normalize(Path::new("./src/../src/a.rs")),
            PathBuf::from("src/a.rs")
        );
        assert_eq!(normalize(Path::new("a/b/../c")), PathBuf::from("a/c"));
    }
}
