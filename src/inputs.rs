//! Input resolution: classify each path argument (file, folder, or glob) and
//! unfold it to a deduped, sorted set of source files for the strip pipeline.
//!
//! Folders are walked `.gitignore`-aware; globs are expanded against the
//! filesystem (file matches contribute themselves, directory matches are walked);
//! query files (`*.cty.{yaml,yml}`) are recognized and skipped — reserved for
//! unfolding in a later task. Paths are lexically normalized so a file reached by
//! several arguments appears once.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use globset::GlobBuilder;
use ignore::WalkBuilder;

use crate::AppError;

/// Glob metacharacters that mark an argument for expansion.
const GLOB_META: [char; 4] = ['*', '?', '[', '{'];

/// Resolve raw path arguments to a deduped, sorted set of source files.
///
/// Each argument is a source file, a folder (walked `.gitignore`-aware), or a
/// glob. Empty input defaults to the current directory.
///
/// # Errors
///
/// [`AppError::Input`] when a named (non-glob) path does not exist or a glob is
/// malformed. [`AppError::Walk`] from the `ignore` walker.
pub fn resolve(args: &[PathBuf]) -> Result<Vec<PathBuf>, AppError> {
    let default = [PathBuf::from(".")];
    let args = if args.is_empty() { &default[..] } else { args };
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();
    for arg in args {
        resolve_one(arg, &mut out)?;
    }
    Ok(out.into_iter().collect())
}

fn resolve_one(arg: &Path, out: &mut BTreeSet<PathBuf>) -> Result<(), AppError> {
    if is_glob(arg) {
        expand_glob(arg, out)
    } else if arg.is_dir() {
        walk_dir(arg, out)
    } else if arg.is_file() {
        add_file(arg, out);
        Ok(())
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

/// A query file reserved for unfolding (task 11): `*.cty.yaml` / `*.cty.yml`.
pub(crate) fn is_query_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".cty.yaml") || name.ends_with(".cty.yml"))
}

/// Add a single file, skipping reserved query files. Stores the normalized path.
fn add_file(path: &Path, out: &mut BTreeSet<PathBuf>) {
    if is_query_file(path) {
        return;
    }
    out.insert(normalize(path));
}

/// Walk a directory `.gitignore`-aware, adding every file it yields.
fn walk_dir(root: &Path, out: &mut BTreeSet<PathBuf>) -> Result<(), AppError> {
    for entry in WalkBuilder::new(root).build() {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_file()) {
            add_file(entry.path(), out);
        }
    }
    Ok(())
}

/// Expand a glob: walk its literal prefix `.gitignore`-aware and match each
/// entry's normalized path; files contribute themselves, matched directories are
/// walked. Zero matches warn to stderr and continue.
fn expand_glob(pattern: &Path, out: &mut BTreeSet<PathBuf>) -> Result<(), AppError> {
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
            walk_dir(entry.path(), out)?;
        } else if kind.is_some_and(|item| item.is_file()) {
            add_file(entry.path(), out);
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

/// Longest leading path of `pattern` with no glob metacharacter — the directory
/// to root the expansion walk at. Defaults to `.` when the first component is a
/// glob.
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
fn normalize(path: &Path) -> PathBuf {
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
        // a.rs reached directly and via the folder walk -> appears once.
        let files = resolve(&[tmp.path().to_path_buf(), one]).expect("resolve");
        assert_eq!(files.len(), 2, "{files:?}");
    }

    #[test]
    fn resolve_missing_path_errors() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("nope.rs");
        let err = resolve(&[missing]).expect_err("missing must error");
        assert!(matches!(err, AppError::Input(_)), "{err:?}");
    }

    #[test]
    fn resolve_glob_matches_files_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "a.rs", "fn a() {}\n");
        write(tmp.path(), "b.rs", "fn b() {}\n");
        write(tmp.path(), "c.txt", "text\n");
        let files = resolve(&[tmp.path().join("*.rs")]).expect("resolve");
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
        let files = resolve(&[tmp.path().join("crates/*/src")]).expect("resolve");
        assert_eq!(files.len(), 2, "{files:?}");
        assert!(files.iter().all(|path| path.ends_with("src/lib.rs")));
    }

    #[test]
    fn resolve_skips_query_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "real.rs", "fn r() {}\n");
        write(tmp.path(), "api.cty.yaml", "rules: []\n");
        let files = resolve(&[tmp.path().to_path_buf()]).expect("resolve");
        assert_eq!(files.len(), 1, "query file must be skipped: {files:?}");
        assert!(files.first().expect("one").ends_with("real.rs"));
    }

    #[test]
    fn resolve_glob_zero_match_is_not_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path(), "a.txt", "text\n");
        let files = resolve(&[tmp.path().join("*.rs")]).expect("zero match warns, not errors");
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
