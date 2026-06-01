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

/// Per-language configuration block, keyed by language name. Carries category
/// inclusion overrides, plus the optional dynamic-grammar registration and
/// rule-override fields. Built-in languages (rust, php) leave every grammar and
/// rule field `None` / empty and use only `include`. Setting `library_path`
/// marks the entry as a custom grammar; `extend` / `override` point any language
/// at a user rule file. Relative `library_path` / `rules` / `extend` /
/// `override` paths resolve against [`Config::base`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LangConfig {
    /// Category inclusion overrides for this language.
    #[serde(default)]
    pub include: CategorySelection,
    /// Compiled grammar for a dynamic language: one shared library, or a
    /// per-target-triple map (native libraries are not portable across OS/arch).
    /// `Some` marks this entry as a custom grammar to register.
    #[serde(default)]
    pub library_path: Option<LibraryPath>,
    /// Dylib symbol exposing the parser. Defaults to `tree_sitter_<key>`.
    #[serde(default)]
    pub language_symbol: Option<String>,
    /// Metavariable sigil for patterns. Defaults to `$`.
    #[serde(default)]
    pub meta_var_char: Option<char>,
    /// Identifier-safe replacement for grammars that reject `$`.
    #[serde(default)]
    pub expando_char: Option<char>,
    /// File extensions (no dot) a custom grammar claims. Required (non-empty)
    /// when `library_path` is set; unused for built-ins.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Path to the `rules/<lang>.yml` rule file driving a custom grammar's strip
    /// pass. Required when `library_path` is set; ignored for built-ins.
    #[serde(default)]
    pub rules: Option<PathBuf>,
    /// Append this file's rules to the language's set (extend mode). The user
    /// rules run after the existing set (matters only for splice precedence).
    #[serde(default)]
    pub extend: Option<PathBuf>,
    /// Replace the language's rules with this file outright (override mode).
    #[serde(default, rename = "override")]
    pub r#override: Option<PathBuf>,
    /// Post-strip reformatter backend. Absent keeps the raw byte-splice (or the
    /// language's built-in formatter, e.g. Rust's prettyplease). See [`Reformat`].
    #[serde(default)]
    pub reformat: Option<Reformat>,
}

/// Post-strip reformatter selection for a language (the `reformat` key of a
/// `[languages.<lang>]` entry).
///
/// - absent / `reformat = "none"` — keep the raw splice (or the language's
///   built-in formatter, e.g. Rust's prettyplease).
/// - `reformat = "topiary"` — embedded Topiary backend (needs the `topiary`
///   build feature and a Topiary query for the language).
/// - `reformat = { command = ["prettier", "--parser", "php"] }` — shell out to
///   an external formatter: stripped source on stdin, formatted source on stdout.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Reformat {
    /// A named backend selected by a bare string (`"none"` / `"topiary"`).
    Mode(ReformatMode),
    /// A shell-out command vector (argv; no shell interpolation).
    Command {
        /// Program plus arguments. The first element is the executable.
        command: Vec<String>,
    },
}

/// The bare-string reformatter backends.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReformatMode {
    /// Keep the unformatted splice (or a built-in formatter); disables an
    /// inherited backend.
    None,
    /// Embedded Topiary backend.
    Topiary,
}

impl LangConfig {
    /// True when this entry registers a dynamic grammar (`library_path` set).
    #[must_use]
    pub const fn is_dynamic(&self) -> bool {
        self.library_path.is_some()
    }

    /// Resolve the rule-override mode for this entry. `Ok(None)` when neither
    /// `extend` nor `override` is set (the entry only tunes inclusion or
    /// registers a grammar). `Err` (with an actionable message) when both are
    /// set — ambiguity is surfaced, not silently resolved.
    ///
    /// # Errors
    ///
    /// A human-readable reason the entry sets both mode keys.
    pub fn rule_source(&self) -> Result<Option<RuleSource<'_>>, String> {
        match (&self.extend, &self.r#override) {
            (Some(_), Some(_)) => Err("set both `extend` and `override`; choose one".to_owned()),
            (Some(path), None) => Ok(Some(RuleSource::Extend(path))),
            (None, Some(path)) => Ok(Some(RuleSource::Override(path))),
            (None, None) => Ok(None),
        }
    }
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
    /// Per-language settings keyed by language name (e.g. `"rust"`, `"php"`, or
    /// a custom grammar's name). Each entry may override category inclusion,
    /// register a dynamic grammar, and/or extend/override the language's rules.
    #[serde(default)]
    pub languages: HashMap<String, LangConfig>,
    /// Directory the config file lives in. Relative `library_path` / `rules` /
    /// `extend` / `override` paths resolve against it. Set by [`Config::load`],
    /// never deserialized.
    #[serde(skip)]
    pub base: PathBuf,
    /// Runtime kill-switch: when set, every language's reformatter is forced to
    /// `none` regardless of config (the `--no-reformat` CLI flag). Set by the
    /// caller after load, never deserialized.
    #[serde(skip)]
    pub no_reformat: bool,
}

/// Resolved rule-override mode of a [`LangConfig`]: which file, applied how.
pub enum RuleSource<'a> {
    /// Append the file's rules to the language's set.
    Extend(&'a Path),
    /// Replace the language's set with the file's rules.
    Override(&'a Path),
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

    #[test]
    fn language_block_parses_grammar_and_rule_fields() {
        let toml = "\
[languages.jsonc]\n\
libraryPath = \"grammars/jsonc.so\"\n\
languageSymbol = \"tree_sitter_json\"\n\
extensions = [\"jsonc\"]\n\
rules = \"rules/jsonc.yml\"\n\
extend = \"rules/jsonc-extra.yml\"\n\
[languages.jsonc.include]\n\
tests = true\n";
        let config: Config = toml::from_str(toml).expect("parse");
        let jsonc = config.languages.get("jsonc").expect("jsonc entry");
        assert!(jsonc.is_dynamic(), "libraryPath marks a dynamic grammar");
        assert_eq!(jsonc.language_symbol.as_deref(), Some("tree_sitter_json"));
        assert_eq!(jsonc.extensions, vec!["jsonc".to_owned()]);
        assert_eq!(jsonc.rules.as_deref(), Some(Path::new("rules/jsonc.yml")));
        assert_eq!(jsonc.include.tests, Some(true));
        assert!(
            matches!(jsonc.rule_source(), Ok(Some(RuleSource::Extend(_)))),
            "extend resolves to a rule source"
        );
    }

    #[test]
    fn builtin_language_block_is_not_dynamic() {
        let config: Config =
            toml::from_str("[languages.rust.include]\ncomments = true\n").expect("parse");
        let rust = config.languages.get("rust").expect("rust entry");
        assert!(!rust.is_dynamic(), "no libraryPath: not a grammar");
        assert!(
            matches!(rust.rule_source(), Ok(None)),
            "no extend/override: no rule source"
        );
    }

    #[test]
    fn reformat_parses_named_mode() {
        let config: Config =
            toml::from_str("[languages.rust]\nreformat = \"topiary\"\n").expect("parse");
        let rust = config.languages.get("rust").expect("rust entry");
        assert!(
            matches!(rust.reformat, Some(Reformat::Mode(ReformatMode::Topiary))),
            "reformat string did not parse as a named mode"
        );
    }

    #[test]
    fn reformat_parses_command_vector() {
        let toml = "[languages.typescript]\nreformat = { command = [\"prettier\", \"--parser\", \"typescript\"] }\n";
        let config: Config = toml::from_str(toml).expect("parse");
        let entry = config.languages.get("typescript").expect("ts entry");
        match &entry.reformat {
            Some(Reformat::Command { command }) => {
                assert_eq!(command, &["prettier", "--parser", "typescript"]);
            }
            other => panic!("expected a command vector, got {other:?}"),
        }
    }

    #[test]
    fn reformat_absent_is_none() {
        let config: Config =
            toml::from_str("[languages.php.include]\ntests = true\n").expect("parse");
        let php = config.languages.get("php").expect("php entry");
        assert!(php.reformat.is_none(), "absent reformat should be None");
    }

    #[test]
    fn both_extend_and_override_is_a_rule_source_error() {
        let config: Config =
            toml::from_str("[languages.rust]\nextend = \"a.yml\"\noverride = \"b.yml\"\n")
                .expect("parse");
        let rust = config.languages.get("rust").expect("rust entry");
        assert!(rust.rule_source().is_err(), "both keys rejected");
    }
}
