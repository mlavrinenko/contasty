use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const DEFAULT_CONFIG_NAME: &str = "contasty.toml";

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub compact: CompactConfig,
    /// User-supplied dynamic tree-sitter grammars, keyed by language name. The
    /// key is the language identifier a rule file's `language:` must name and
    /// the dylib symbol defaults to `tree_sitter_<key>`.
    #[serde(default, rename = "customLanguages")]
    pub custom_languages: HashMap<String, CustomLanguage>,
    /// Per-language rule overrides, keyed by language name (the table key). Each
    /// entry either extends or replaces that language's embedded rules with a
    /// user file. Paths resolve against [`Config::base`], like `customLanguages`.
    #[serde(default)]
    pub rules: HashMap<String, RuleOverride>,
    /// Directory the config file lives in. Relative `library_path` / `rules`
    /// paths resolve against it. Set by [`Config::load`], never deserialized.
    #[serde(skip)]
    pub base: PathBuf,
}

/// One `[rules.<lang>]` entry: point a built-in (or dynamic) language at a user
/// rule file that either extends or replaces its standard rules. Exactly one of
/// `extend` / `override` must be set — both (or neither) is a config error,
/// surfaced rather than silently resolved.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleOverride {
    /// Append this file's rules to the language's embedded set. The user rules
    /// run after the built-ins (matters only for splice precedence).
    #[serde(default)]
    pub extend: Option<PathBuf>,
    /// Ignore the embedded rules; this file is the whole set for the language.
    #[serde(default, rename = "override")]
    pub r#override: Option<PathBuf>,
}

/// Resolved mode of a [`RuleOverride`]: which file, applied which way.
pub enum RuleSource<'a> {
    /// Append the file's rules to the embedded set.
    Extend(&'a Path),
    /// Replace the embedded set with the file's rules.
    Override(&'a Path),
}

impl RuleOverride {
    /// Resolve the single mode key. `Err` (with an actionable message) when both
    /// or neither of `extend` / `override` is set.
    ///
    /// # Errors
    ///
    /// A human-readable reason the entry is ambiguous or empty.
    pub fn source(&self) -> Result<RuleSource<'_>, String> {
        match (&self.extend, &self.r#override) {
            (Some(_), Some(_)) => Err("set both `extend` and `override`; choose one".to_owned()),
            (Some(path), None) => Ok(RuleSource::Extend(path)),
            (None, Some(path)) => Ok(RuleSource::Override(path)),
            (None, None) => Err("set neither `extend` nor `override`".to_owned()),
        }
    }
}

/// One entry of the `[customLanguages]` table: an `ast-grep-dynamic` grammar
/// plus the contasty rule file driving its strip pass. Field names mirror
/// `ast_grep_dynamic::CustomLang` so a config carries straight over, with the
/// extra `rules` pointer contasty needs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomLanguage {
    /// Compiled grammar: one shared library, or a per-target-triple map (native
    /// libraries are not portable across OS/arch).
    pub library_path: LibraryPath,
    /// Dylib symbol exposing the parser. Defaults to `tree_sitter_<key>`.
    #[serde(default)]
    pub language_symbol: Option<String>,
    /// Metavariable sigil for patterns. Defaults to `$`.
    #[serde(default)]
    pub meta_var_char: Option<char>,
    /// Identifier-safe replacement for grammars that reject `$`.
    #[serde(default)]
    pub expando_char: Option<char>,
    /// File extensions (no dot) this grammar claims.
    pub extensions: Vec<String>,
    /// Path to the `rules/<lang>.yml` rule file, relative to the config file.
    pub rules: PathBuf,
}

/// Where a custom grammar's shared library lives. Mirrors
/// `ast_grep_dynamic::LibraryPath` but derives `Debug` so it can sit in
/// [`Config`].
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LibraryPath {
    /// A single library used on every target.
    Single(PathBuf),
    /// A target-triple → library map (e.g. `x86_64-unknown-linux-gnu`).
    Platform(HashMap<String, PathBuf>),
}

#[derive(Debug, Deserialize)]
pub struct CompactConfig {
    #[serde(default = "default_min_bytes")]
    pub elide_min_bytes: usize,
    #[serde(default = "default_max_string_bytes")]
    pub max_string_bytes: usize,
}

impl Default for CompactConfig {
    fn default() -> Self {
        Self {
            elide_min_bytes: default_min_bytes(),
            max_string_bytes: default_max_string_bytes(),
        }
    }
}

const fn default_min_bytes() -> usize {
    0
}

const fn default_max_string_bytes() -> usize {
    256
}

impl Config {
    pub fn load(from_path: Option<&Path>, working_dir: &Path) -> Self {
        let path = from_path.map_or_else(|| working_dir.join(DEFAULT_CONFIG_NAME), PathBuf::from);
        let mut config = Self::load_file(&path).unwrap_or_default();
        config.base = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| working_dir.to_path_buf(), Path::to_path_buf);
        config
    }

    fn load_file(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn config_defaults_when_file_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = Config::load(None, dir.path());
        assert_eq!(config.compact.elide_min_bytes, 0);
    }

    #[test]
    fn config_loads_from_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        let mut f = std::fs::File::create(&path).expect("create");
        writeln!(f, "[compact]\nelide_min_bytes = 256").expect("write");

        let config = Config::load(Some(&path), dir.path());
        assert_eq!(config.compact.elide_min_bytes, 256);
    }

    #[test]
    fn config_defaults_when_invalid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        std::fs::write(&path, "not valid toml {{{").expect("write");

        let config = Config::load(Some(&path), dir.path());
        assert_eq!(config.compact.elide_min_bytes, default_min_bytes());
    }
}
