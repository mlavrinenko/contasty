use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const DEFAULT_CONFIG_NAME: &str = "contasty.toml";

/// Per-category inclusion flags. `Some(true)` = include, `Some(false)` = exclude,
/// `None` = inherit from a lower-priority layer.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategorySelection {
    #[serde(default)]
    pub comments: Option<bool>,
    #[serde(default)]
    pub imports: Option<bool>,
    #[serde(default)]
    pub tests: Option<bool>,
}

impl CategorySelection {
    /// Built-in defaults: comments excluded, tests excluded, imports included.
    pub const BUILTIN: Self = Self {
        comments: Some(false),
        imports: Some(true),
        tests: Some(false),
    };

    /// Overlay `higher` onto `self`: any `Some` in `higher` wins.
    #[must_use]
    pub fn overlay(self, higher: Self) -> Self {
        Self {
            comments: higher.comments.or(self.comments),
            imports: higher.imports.or(self.imports),
            tests: higher.tests.or(self.tests),
        }
    }
}

/// Per-language configuration block, currently just inclusion overrides.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LangConfig {
    #[serde(default)]
    pub include: CategorySelection,
}

/// Concrete per-file drop flags after all layers are resolved.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedDrops {
    pub drop_tests: bool,
    pub drop_comments: bool,
    pub drop_imports: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub compact: CompactConfig,
    /// Cross-language category inclusion defaults. Individual categories are
    /// `None` until set, falling back to the built-in defaults.
    #[serde(default)]
    pub include: CategorySelection,
    /// Per-language settings keyed by language name (e.g. `"rust"`, `"php"`).
    /// Each entry may override category inclusion for that language.
    #[serde(default)]
    pub languages: HashMap<String, LangConfig>,
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
    /// Resolve the effective drop flags for a language by layering:
    /// built-in < `[include]` defaults < `[languages.<lang>.include]` < `cli`.
    #[must_use]
    pub fn resolve_selection(&self, lang: &str, cli: CategorySelection) -> ResolvedDrops {
        let per_lang = self
            .languages
            .get(lang)
            .map_or(CategorySelection::default(), |l| l.include);
        let s = CategorySelection::BUILTIN
            .overlay(self.include)
            .overlay(per_lang)
            .overlay(cli);
        ResolvedDrops {
            drop_comments: !s.comments.unwrap_or(false),
            drop_imports: !s.imports.unwrap_or(true),
            drop_tests: !s.tests.unwrap_or(false),
        }
    }

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

    fn no_cli() -> CategorySelection {
        CategorySelection::default()
    }

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

    #[test]
    fn builtin_defaults_apply_when_no_config_or_cli() {
        let config = Config::default();
        let drops = config.resolve_selection("rust", no_cli());
        assert!(drops.drop_comments, "comments excluded by default");
        assert!(!drops.drop_imports, "imports included by default");
        assert!(drops.drop_tests, "tests excluded by default");
    }

    #[test]
    fn include_cross_language_defaults_apply_to_all_langs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        std::fs::write(&path, "[include]\ncomments = true\n").expect("write");

        let config = Config::load(Some(&path), dir.path());
        assert!(!config.resolve_selection("rust", no_cli()).drop_comments);
        assert!(!config.resolve_selection("php", no_cli()).drop_comments);
    }

    #[test]
    fn include_per_language_overrides_cross_language_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        std::fs::write(
            &path,
            "[include]\ncomments = false\n[languages.rust.include]\ncomments = true\n",
        )
        .expect("write");

        let config = Config::load(Some(&path), dir.path());
        assert!(
            !config.resolve_selection("rust", no_cli()).drop_comments,
            "rust: comments included"
        );
        assert!(
            config.resolve_selection("php", no_cli()).drop_comments,
            "php: comments excluded"
        );
    }

    #[test]
    fn unknown_language_falls_back_to_cross_language_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        std::fs::write(&path, "[include]\nimports = false\n").expect("write");

        let config = Config::load(Some(&path), dir.path());
        assert!(
            config
                .resolve_selection("javascript", no_cli())
                .drop_imports
        );
    }

    #[test]
    fn cli_override_beats_per_language_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        std::fs::write(&path, "[languages.rust.include]\ncomments = true\n").expect("write");

        let config = Config::load(Some(&path), dir.path());
        let cli = CategorySelection {
            comments: Some(false),
            ..CategorySelection::default()
        };
        assert!(
            config.resolve_selection("rust", cli).drop_comments,
            "CLI wins over per-lang config"
        );
    }

    #[test]
    fn cli_override_is_global() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        std::fs::write(&path, "[languages.php.include]\nimports = false\n").expect("write");

        let config = Config::load(Some(&path), dir.path());
        let cli = CategorySelection {
            imports: Some(true),
            ..CategorySelection::default()
        };
        assert!(
            !config.resolve_selection("php", cli).drop_imports,
            "CLI include beats per-lang exclude"
        );
        assert!(
            !config.resolve_selection("rust", cli).drop_imports,
            "CLI applies globally"
        );
    }

    #[test]
    fn include_section_round_trips_toml() {
        let toml = "\
[include]\n\
comments = false\n\
imports  = true\n\
tests    = false\n\
[languages.rust.include]\n\
comments = true\n\
[languages.php.include]\n\
imports = false\n";
        let config: Config = toml::from_str(toml).expect("parse");
        assert_eq!(config.include.comments, Some(false));
        assert_eq!(config.include.imports, Some(true));
        assert_eq!(
            config.languages.get("rust").map(|l| l.include.comments),
            Some(Some(true))
        );
        assert_eq!(
            config.languages.get("php").map(|l| l.include.imports),
            Some(Some(false))
        );
    }
}
