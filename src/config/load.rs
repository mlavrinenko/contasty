//! Two-layer config load: `<global>/config.toml` under `<project>/.contasty/
//! config.toml` (or `--config`'s path), project winning on a shared key.
//!
//! Each layer deserializes into [`RawConfig`] — every field optional, so a
//! layer that sets nothing is indistinguishable from a missing file. Each
//! [`LangConfig`] path field is resolved to an absolute path against its own
//! layer's config-file directory immediately after deserializing that layer,
//! before the merge — so `with_config` / `dynamic::register` /
//! `apply_overrides` never need to know which directory (or which layer)
//! defined an entry.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{CompactConfig, Config, LangConfig, LibraryPath, StripConfig};

/// Filename shared by both layers; only the parent directory differs.
const CONFIG_FILE_NAME: &str = "config.toml";
/// Project-relative directory the default project config lives under.
const PROJECT_CONFIG_DIR: &str = ".contasty";

/// All-optional deserialization form of one config layer. `Option` (rather
/// than a default value) lets the merge tell "this layer set it" from "this
/// layer is silent, inherit the other layer".
#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    compact: Option<CompactConfig>,
    #[serde(default)]
    strip: Option<StripConfig>,
    #[serde(default)]
    languages: Option<HashMap<String, LangConfig>>,
}

impl Config {
    /// Load and merge the two config layers, project over global:
    ///
    /// 1. Global: `<global_dir>/config.toml`, when `global_dir` is `Some`.
    /// 2. Project: `cli_config` when given, else
    ///    `<project_dir>/.contasty/config.toml`.
    ///
    /// A missing or unparsable layer is silently treated as empty (matching
    /// the pre-layering behaviour of a missing/invalid config file).
    /// `compact` replaces wholesale when the project sets it, else inherits
    /// the global value, else the built-in default; `strip` is
    /// `project.or(global)`; `languages` unions by key with the project entry
    /// winning wholesale on a shared key.
    #[must_use]
    pub fn load(cli_config: Option<&Path>, project_dir: &Path, global_dir: Option<&Path>) -> Self {
        let global = global_dir.and_then(|dir| load_raw(&dir.join(CONFIG_FILE_NAME), dir));
        let project_path = cli_config.map_or_else(
            || project_dir.join(PROJECT_CONFIG_DIR).join(CONFIG_FILE_NAME),
            PathBuf::from,
        );
        let project = load_raw(&project_path, project_dir);
        merge(global, project)
    }
}

/// Merge two optional raw layers into a resolved `Config`, project over
/// global.
fn merge(global: Option<RawConfig>, project: Option<RawConfig>) -> Config {
    let global = global.unwrap_or_default();
    let project = project.unwrap_or_default();
    let mut languages = global.languages.unwrap_or_default();
    if let Some(project_languages) = project.languages {
        languages.extend(project_languages);
    }
    Config {
        compact: project.compact.or(global.compact).unwrap_or_default(),
        strip: project.strip.or(global.strip),
        languages,
    }
}

/// Read and parse one layer, resolving every `LangConfig` path field to
/// absolute against the config file's own directory (falling back to
/// `working_dir` when the path has no parent component, e.g. a bare
/// filename). `None` on any read/parse failure — a missing or broken layer is
/// silently empty.
fn load_raw(path: &Path, working_dir: &Path) -> Option<RawConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut raw: RawConfig = toml::from_str(&content).ok()?;
    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(working_dir);
    if let Some(languages) = raw.languages.as_mut() {
        for cfg in languages.values_mut() {
            absolutize_lang_paths(cfg, base);
        }
    }
    Some(raw)
}

/// Resolve every path field of one `[languages.<lang>]` entry to absolute
/// against `base`.
fn absolutize_lang_paths(cfg: &mut LangConfig, base: &Path) {
    absolutize_opt(base, &mut cfg.rules);
    absolutize_opt(base, &mut cfg.extend);
    absolutize_opt(base, &mut cfg.r#override);
    match &mut cfg.library_path {
        Some(LibraryPath::Single(path)) => {
            let resolved = absolutize(base, path);
            *path = resolved;
        }
        Some(LibraryPath::Platform(map)) => {
            for path in map.values_mut() {
                let resolved = absolutize(base, path);
                *path = resolved;
            }
        }
        None => {}
    }
}

fn absolutize_opt(base: &Path, field: &mut Option<PathBuf>) {
    if let Some(path) = field {
        let resolved = absolutize(base, path);
        *path = resolved;
    }
}

/// `path` unchanged if already absolute, else joined onto `base`.
fn absolutize(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

#[cfg(test)]
#[path = "load_tests.rs"]
mod tests;
