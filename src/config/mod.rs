//! Configuration types plus the resolved, in-memory `Config`.
//!
//! A project's `.contasty/config.toml` is layered over the XDG global
//! `<global>/config.toml` (project wins on a shared key); see [`load`] for the
//! merge itself. Every [`LangConfig`] path field is resolved to an absolute
//! path against its own config file's directory as part of that load, so
//! nothing downstream needs to know which layer — or which directory —
//! defined an entry.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

mod load;

/// A strip-category: one kind of code contasty can strip.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Comments,
    Imports,
    Tests,
    Body,
}

/// Bit-set of categories to strip. Replaces the old per-category inclusion
/// flags with a single set: a bit ON means "strip this category".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StripSet(u8);

impl StripSet {
    pub const COMMENTS: u8 = 1;
    pub const IMPORTS: u8 = 2;
    pub const TESTS: u8 = 4;
    pub const BODY: u8 = 8;
    const ALL_BITS: u8 = 0b1111;

    /// Default strip set: comments, imports, and bodies stripped; tests kept.
    pub const DEFAULT: Self = Self(Self::COMMENTS | Self::IMPORTS | Self::BODY);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn all() -> Self {
        Self(Self::ALL_BITS)
    }

    pub const fn contains(self, bit: u8) -> bool {
        self.0 & bit != 0
    }

    pub const fn insert(self, bit: u8) -> Self {
        Self(self.0 | bit)
    }

    pub const fn remove(self, bit: u8) -> Self {
        Self(self.0 & !bit)
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    const fn bit_for(cat: Category) -> u8 {
        match cat {
            Category::Comments => Self::COMMENTS,
            Category::Imports => Self::IMPORTS,
            Category::Tests => Self::TESTS,
            Category::Body => Self::BODY,
        }
    }

    pub const fn drop_comments(self) -> bool {
        self.contains(Self::COMMENTS)
    }
    pub const fn drop_imports(self) -> bool {
        self.contains(Self::IMPORTS)
    }
    pub const fn drop_tests(self) -> bool {
        self.contains(Self::TESTS)
    }
    pub const fn drop_bodies(self) -> bool {
        self.contains(Self::BODY)
    }

    /// Parse a comma-separated category list with `!` negation.
    ///
    /// `all` / `everything` resets to all four categories; `none` resets to
    /// empty. A `!` prefix removes a category from the set built so far.
    /// Tokens are processed left to right.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message for unknown category names.
    pub fn parse_list(input: &str) -> Result<Self, String> {
        let mut set = Self::empty();
        for token in input.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if let Some(rest) = token.strip_prefix('!') {
                let cat = parse_category(rest)?;
                set = set.remove(Self::bit_for(cat));
                continue;
            }
            match token {
                "all" | "everything" => set = Self::all(),
                "none" => set = Self::empty(),
                _ => {
                    let cat = parse_category(token)?;
                    set = set.insert(Self::bit_for(cat));
                }
            }
        }
        Ok(set)
    }
}

fn parse_category(name: &str) -> Result<Category, String> {
    match name {
        "comments" => Ok(Category::Comments),
        "imports" => Ok(Category::Imports),
        "tests" => Ok(Category::Tests),
        "body" => Ok(Category::Body),
        _ => Err(format!("unknown strip category: `{name}`")),
    }
}

/// Wrapper for deserializing a `StripSet` from a TOML / YAML sequence of
/// category name strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StripConfig(pub StripSet);

impl<'de> Deserialize<'de> for StripConfig {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Vis;
        impl<'de> serde::de::Visitor<'de> for Vis {
            type Value = StripConfig;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a list of strip categories")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut set = StripSet::empty();
                while let Some(cat) = seq.next_element::<Category>()? {
                    set = set.insert(StripSet::bit_for(cat));
                }
                Ok(StripConfig(set))
            }
        }
        de.deserialize_seq(Vis)
    }
}

/// Concrete per-file drop flags after all layers are resolved.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct ResolvedDrops {
    pub drop_tests: bool,
    pub drop_comments: bool,
    pub drop_imports: bool,
    pub drop_bodies: bool,
}

/// Per-file strip inputs carried from `resolve` to `collect`.
///
/// `cli` is the strip set the path's CLI group selected, or `None` when the
/// group set no explicit `--strip` (fall through to config layering). `query`
/// is the strip set a query file declared (empty for plain files); it unions
/// onto the resolved base ("CLI adds to query").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStrip {
    pub cli: Option<StripSet>,
    pub query: StripSet,
}

impl FileStrip {
    #[must_use]
    pub const fn new(cli: Option<StripSet>, query: StripSet) -> Self {
        Self { cli, query }
    }
}

/// Per-language configuration block, keyed by language name.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LangConfig {
    /// Strip-category overrides for this language.
    #[serde(default)]
    pub strip: Option<StripConfig>,
    /// Compiled grammar for a dynamic language.
    #[serde(default)]
    pub library_path: Option<LibraryPath>,
    /// Dylib symbol exposing the parser.
    #[serde(default)]
    pub language_symbol: Option<String>,
    /// Metavariable sigil for patterns.
    #[serde(default)]
    pub meta_var_char: Option<char>,
    /// Identifier-safe replacement for grammars that reject `$`.
    #[serde(default)]
    pub expando_char: Option<char>,
    /// File extensions (no dot) a custom grammar claims.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Path to the `rules/<lang>.yml` rule file. Absolute once loaded via
    /// [`Config::load`] — resolved against its defining config file's directory.
    #[serde(default)]
    pub rules: Option<PathBuf>,
    /// Append this file's rules to the language's set. Absolute once loaded.
    #[serde(default)]
    pub extend: Option<PathBuf>,
    /// Replace the language's rules with this file outright. Absolute once
    /// loaded.
    #[serde(default, rename = "override")]
    pub r#override: Option<PathBuf>,
}

impl LangConfig {
    #[must_use]
    pub const fn is_dynamic(&self) -> bool {
        self.library_path.is_some()
    }

    /// Resolve the rule-override mode for this entry.
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

/// Resolved rule-override mode of a [`LangConfig`].
pub enum RuleSource<'a> {
    Extend(&'a Path),
    Override(&'a Path),
}

/// Where a custom grammar's shared library lives. Path(s) absolute once
/// loaded via [`Config::load`].
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LibraryPath {
    Single(PathBuf),
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

/// Fully resolved, two-layer-merged configuration. Built by [`Config::load`];
/// every [`LangConfig`] path field it holds is already absolute.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub compact: CompactConfig,
    /// Cross-language strip-category defaults.
    #[serde(default)]
    pub strip: Option<StripConfig>,
    /// Per-language settings keyed by language name.
    #[serde(default)]
    pub languages: HashMap<String, LangConfig>,
}

impl Config {
    /// Built-in default strip set: comments, imports, and bodies stripped;
    /// tests kept.
    pub const BUILTIN_STRIP: StripSet = StripSet::empty()
        .insert(StripSet::COMMENTS)
        .insert(StripSet::IMPORTS)
        .insert(StripSet::BODY);

    /// Resolve the effective drop flags for a file, applying the full layering:
    /// built-in < `[strip]` < `[languages.<lang>.strip]` < CLI-per-path, then
    /// union the query's own strip set.
    ///
    /// When the CLI group set no explicit `--strip` (`cli == None`), the
    /// config-layered default for the language is the base; an explicit CLI
    /// strip replaces it. Either way the query strip is unioned on top.
    #[must_use]
    pub fn resolve_drops(&self, lang: &str, strip: FileStrip) -> ResolvedDrops {
        let base = strip.cli.unwrap_or_else(|| self.resolve_config_strip(lang));
        let effective = base.union(strip.query);
        ResolvedDrops {
            drop_comments: effective.drop_comments(),
            drop_imports: effective.drop_imports(),
            drop_tests: effective.drop_tests(),
            drop_bodies: effective.drop_bodies(),
        }
    }

    /// Resolve the config-layered strip set for a language:
    /// built-in < `[strip]` < `[languages.<lang>.strip]`.
    #[must_use]
    pub fn resolve_config_strip(&self, lang: &str) -> StripSet {
        let base = Self::BUILTIN_STRIP;
        let cross = self.strip.map_or(base, |sc| sc.0);
        self.languages
            .get(lang)
            .and_then(|l| l.strip)
            .map_or(cross, |sc| sc.0)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
