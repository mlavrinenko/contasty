//! Language registry and source-stripping core.
//!
//! Adding a language is a two-step recipe and needs no per-language Rust logic:
//!
//! 1. Drop a `rules/<lang>.yml` ast-grep rule set beside this module.
//! 2. Register it in [`Registry::new`] with a display name (and an optional
//!    post-strip formatter).
//!
//! Each rule selects an anchor node, optionally descends into a named field,
//! then maps to an [`Action`]. Matching is delegated to ast-grep; this module
//! owns only the field descent, attribute expansion, and byte-range splicing.

use std::path::Path;
use std::str::FromStr;

use ast_grep_config::{DeserializeEnv, RuleCore, SerializableRule, SerializableRuleCore};
use ast_grep_core::AstGrep;
use ast_grep_core::tree_sitter::StrDoc;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::AppError;
use crate::config::{CompactConfig, Config};
use crate::lang::dynamic::Lang;
use crate::lang::reformat::Reformatter;

mod dynamic;
mod overrides;
mod reformat;
mod shellout;
mod splice;
#[cfg(feature = "topiary")]
mod topiary;

use splice::splice;

type Doc = StrDoc<Lang>;
type AstNode<'r> = ast_grep_core::Node<'r, Doc>;

/// A registered language: a tree-sitter grammar plus its compiled rule set.
pub struct Language {
    /// Markdown fence info-string (e.g. `"rust"`).
    pub name: &'static str,
    lang: Lang,
    rules: Vec<CompiledRule>,
    /// Post-strip reformatter applied after splicing. `None` by default; the
    /// `reformat` config key opts a language into Topiary or a shell-out
    /// command. A failure falls back to the unformatted splice rather than
    /// failing the whole file.
    reformat: Reformatter,
}

/// One compiled strip rule: an ast-grep matcher plus what to do with its hits.
struct CompiledRule {
    matcher: RuleCore,
    action: Action,
    /// Named field to descend into on the matched node before acting. `None`
    /// acts on the matched node itself; a missing field skips the match.
    field: Option<String>,
    /// When this rule runs relative to the caller's drop flags.
    gate: Gate,
    /// Minimum captured byte length for the match to count. `None` means zero.
    min_bytes: Option<Threshold>,
    /// Absorb adjacent attribute siblings + the decorated item into one range.
    expand_attributes: bool,
}

/// Which gated rule groups run, plus the size thresholds for this strip pass.
#[allow(clippy::struct_excessive_bools)]
struct StripOptions<'cfg> {
    drop_tests: bool,
    drop_comments: bool,
    drop_imports: bool,
    drop_bodies: bool,
    compact: &'cfg CompactConfig,
}

/// What to do with a captured byte range.
#[derive(Clone, Copy)]
enum Action {
    /// Replace with `ELISION`.
    Elide,
    /// Remove the range plus one trailing newline if present.
    Delete,
    /// Replace with a string-truncation marker.
    TruncateString,
}

// --- Rule file schema (deserialized from `rules/<lang>.yml`) ---------------

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
/// A `rules/<lang>.yml` file: a target language plus its ordered strip rules.
struct RuleFile {
    language: String,
    rules: Vec<RuleSpec>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
/// One strip rule: an ast-grep matcher plus what to do with each hit.
struct RuleSpec {
    action: RuleAction,
    rule: SerializableRule,
    #[serde(default)]
    field: Option<String>,
    #[serde(default)]
    when: Gate,
    #[serde(default)]
    min_bytes: Option<Threshold>,
    #[serde(default)]
    expand_attributes: bool,
}

#[derive(Deserialize, Clone, Copy, JsonSchema)]
#[serde(rename_all = "kebab-case")]
/// What to do with each matched range.
enum RuleAction {
    Elide,
    Delete,
    Truncate,
}

impl From<RuleAction> for Action {
    fn from(action: RuleAction) -> Self {
        match action {
            RuleAction::Elide => Self::Elide,
            RuleAction::Delete => Self::Delete,
            RuleAction::Truncate => Self::TruncateString,
        }
    }
}

#[derive(Deserialize, Clone, Copy, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
/// Which drop flag, if any, gates this rule.
enum Gate {
    #[default]
    Always,
    Body,
    Tests,
    Comments,
    Imports,
}

impl Gate {
    const fn enabled(self, opts: &StripOptions) -> bool {
        match self {
            Self::Always => true,
            Self::Body => opts.drop_bodies,
            Self::Tests => opts.drop_tests,
            Self::Comments => opts.drop_comments,
            Self::Imports => opts.drop_imports,
        }
    }
}

#[derive(Deserialize, Clone, Copy, JsonSchema)]
#[serde(rename_all = "kebab-case")]
/// Which configured byte threshold a match must clear to count.
enum Threshold {
    ElideMin,
    MaxString,
}

impl Threshold {
    const fn resolve(self, compact: &CompactConfig) -> usize {
        match self {
            Self::ElideMin => compact.elide_min_bytes,
            Self::MaxString => compact.max_string_bytes,
        }
    }
}

/// JSON Schema (Draft 2020-12) for the `rules/<lang>.yml` format, pretty-printed
/// with a trailing newline. The rule subtree is composed from `ast-grep-config`'s
/// own `SerializableRule` schema. Backs `schemas/contasty-rules.schema.json`;
/// regenerate with `just gen-schema`, drift-guarded by the `schema_in_sync` test.
#[must_use]
pub fn rules_schema_json() -> String {
    let schema = schemars::schema_for!(RuleFile);
    let mut json = serde_json::to_string_pretty(&schema).unwrap_or_default();
    json.push('\n');
    json
}

impl Language {
    /// Compile a language descriptor from an embedded ast-grep rule set.
    ///
    /// # Errors
    ///
    /// [`AppError::RuleParse`] if the YAML is malformed, [`AppError::Rule`] if
    /// the declared language is unknown or any rule fails to compile against the
    /// grammar.
    fn from_rules(name: &'static str, yaml: &str, reformat: Reformatter) -> Result<Self, AppError> {
        let file: RuleFile = serde_yaml::from_str(yaml)?;
        let lang = Lang::from_str(&file.language)?;
        let rules = file
            .rules
            .into_iter()
            .map(|spec| compile_rule(lang, spec))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            name,
            lang,
            rules,
            reformat,
        })
    }

    /// Strip elidable nodes from `source`. The `drop_*` flags gate the body,
    /// test, comment, and import rules respectively.
    ///
    /// # Errors
    ///
    /// [`AppError::ParseFailed`] if tree-sitter cannot produce a parse tree.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn strip(
        &self,
        source: &str,
        path: &Path,
        drop_tests: bool,
        drop_comments: bool,
        drop_imports: bool,
        drop_bodies: bool,
        compact: &CompactConfig,
    ) -> Result<String, AppError> {
        let grep = AstGrep::try_new(source, self.lang).map_err(|_| AppError::ParseFailed {
            path: path.to_path_buf(),
        })?;
        let opts = StripOptions {
            drop_tests,
            drop_comments,
            drop_imports,
            drop_bodies,
            compact,
        };
        let ranges = self.collect(&grep, &opts);
        let spliced = splice(source, &ranges);
        Ok(self.reformat.apply(&spliced))
    }

    fn collect(&self, grep: &AstGrep<Doc>, opts: &StripOptions) -> Vec<(usize, usize, Action)> {
        let root = grep.root();
        let mut ranges = Vec::new();
        for rule in &self.rules {
            if !rule.gate.enabled(opts) {
                continue;
            }
            let min = rule
                .min_bytes
                .map_or(0, |threshold| threshold.resolve(opts.compact));
            for matched in root.find_all(&rule.matcher) {
                let Some(node) = descend(matched.get_node(), rule.field.as_deref()) else {
                    continue;
                };
                push_range(&node, rule, min, &mut ranges);
            }
        }
        ranges
    }
}

fn compile_rule(lang: Lang, spec: RuleSpec) -> Result<CompiledRule, AppError> {
    let core = SerializableRuleCore {
        rule: spec.rule,
        constraints: None,
        utils: None,
        transform: None,
        fix: None,
    };
    let matcher = core
        .get_matcher(DeserializeEnv::new(lang))
        .map_err(|err| AppError::Rule(err.to_string()))?;
    Ok(CompiledRule {
        matcher,
        action: spec.action.into(),
        field: spec.field,
        gate: spec.when,
        min_bytes: spec.min_bytes,
        expand_attributes: spec.expand_attributes,
    })
}

/// Descend into `field` of `node`, or return the node itself when no field is
/// requested. A requested-but-absent field yields `None` (skip the match).
fn descend<'r>(node: &AstNode<'r>, field: Option<&str>) -> Option<AstNode<'r>> {
    match field {
        Some(name) => node.field(name),
        None => Some(node.clone()),
    }
}

fn push_range(
    node: &AstNode<'_>,
    rule: &CompiledRule,
    min: usize,
    out: &mut Vec<(usize, usize, Action)>,
) {
    if rule.expand_attributes {
        let (start, end) = expand_attribute_to_item(node);
        if start < end {
            out.push((start, end, rule.action));
        }
        return;
    }
    let range = node.range();
    if range.end - range.start >= min {
        out.push((range.start, range.end, rule.action));
    }
}

/// Given an attribute node, walk to its decorated item, absorbing any adjacent
/// attribute siblings. Returns the byte range covering the whole `#[a] #[b]
/// item` group.
fn expand_attribute_to_item(attr: &AstNode<'_>) -> (usize, usize) {
    let mut start = attr.range().start;
    let mut cursor = attr.clone();
    while let Some(prev) = named_prev(&cursor) {
        if !is_attribute(&prev) {
            break;
        }
        start = prev.range().start;
        cursor = prev;
    }
    let mut end = attr.range().end;
    let mut cursor = attr.clone();
    while let Some(next) = named_next(&cursor) {
        end = next.range().end;
        let absorb = is_attribute(&next);
        cursor = next;
        if !absorb {
            break;
        }
    }
    (start, end)
}

fn is_attribute(node: &AstNode<'_>) -> bool {
    node.kind().as_ref() == "attribute_item"
}

fn named_prev<'r>(node: &AstNode<'r>) -> Option<AstNode<'r>> {
    let mut prev = node.prev();
    while let Some(candidate) = prev {
        if candidate.is_named() {
            return Some(candidate);
        }
        prev = candidate.prev();
    }
    None
}

fn named_next<'r>(node: &AstNode<'r>) -> Option<AstNode<'r>> {
    let mut next = node.next();
    while let Some(candidate) = next {
        if candidate.is_named() {
            return Some(candidate);
        }
        next = candidate.next();
    }
    None
}

/// Set of languages contasty knows how to strip.
pub struct Registry {
    langs: Vec<Language>,
}

impl Registry {
    /// Build the registry with every supported language.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Rule`] / [`AppError::RuleParse`] if any embedded rule
    /// set fails to parse or compile. This is effectively a build-time bug.
    pub fn new() -> Result<Self, AppError> {
        // Every built-in registers with `Reformatter::None`: the engine ships no
        // per-language formatter; reformatting is opt-in (see reformat.rs).
        let builtin = |name, yaml| Language::from_rules(name, yaml, Reformatter::None);
        Ok(Self {
            langs: vec![
                builtin("rust", include_str!("rules/rust.yml"))?,
                builtin("php", include_str!("rules/php.yml"))?,
                builtin("typescript", include_str!("rules/typescript.yml"))?,
                builtin("tsx", include_str!("rules/tsx.yml"))?,
                builtin("javascript", include_str!("rules/javascript.yml"))?,
                builtin("python", include_str!("rules/python.yml"))?,
                builtin("go", include_str!("rules/go.yml"))?,
                builtin("java", include_str!("rules/java.yml"))?,
                builtin("csharp", include_str!("rules/csharp.yml"))?,
                builtin("ruby", include_str!("rules/ruby.yml"))?,
                builtin("cpp", include_str!("rules/cpp.yml"))?,
                builtin("c", include_str!("rules/c.yml"))?,
                builtin("kotlin", include_str!("rules/kotlin.yml"))?,
                builtin("swift", include_str!("rules/swift.yml"))?,
                builtin("scala", include_str!("rules/scala.yml"))?,
                builtin("bash", include_str!("rules/bash.yml"))?,
                builtin("lua", include_str!("rules/lua.yml"))?,
                builtin("dart", include_str!("rules/dart.yml"))?,
                builtin("elixir", include_str!("rules/elixir.yml"))?,
                builtin("haskell", include_str!("rules/haskell.yml"))?,
                builtin("nix", include_str!("rules/nix.yml"))?,
                builtin("solidity", include_str!("rules/solidity.yml"))?,
                builtin("json", include_str!("rules/json.yml"))?,
                builtin("yaml", include_str!("rules/yaml.yml"))?,
                builtin("html", include_str!("rules/html.yml"))?,
                builtin("css", include_str!("rules/css.yml"))?,
                builtin("hcl", include_str!("rules/hcl.yml"))?,
            ],
        })
    }

    /// Build the registry with the built-ins plus every configured custom
    /// grammar, then apply the per-language rule overrides. Registers the dynamic
    /// grammars (`[languages.<lang>]` entries with a `libraryPath`, once),
    /// compiles each one's rule file, extends/replaces any language whose entry
    /// sets `extend` / `override`, then resolves each entry's `reformat` backend
    /// (unless `--no-reformat`). Paths resolve against the config file's dir.
    ///
    /// # Errors
    ///
    /// [`AppError::CustomLang`] if a grammar fails to load, lacks `extensions` /
    /// `rules`, or its rule file is unreadable; [`AppError::Config`] if an
    /// `extend` / `override` entry is malformed (both mode keys, unknown
    /// language, unreadable file, mismatched `language:`), or a `reformat` entry
    /// names an unknown language, an empty command, or unavailable Topiary;
    /// [`AppError::Rule`] / [`AppError::RuleParse`] if any rule file is malformed
    /// or references kinds the grammar lacks.
    pub fn with_config(config: &Config) -> Result<Self, AppError> {
        let mut registry = Self::new()?;
        let base = config.base.as_path();
        dynamic::register(base, &config.languages)?;
        for (name, cfg) in &config.languages {
            if !cfg.is_dynamic() {
                continue;
            }
            let rules = cfg.rules.as_ref().ok_or_else(|| {
                AppError::CustomLang(format!("languages.{name}: custom grammar needs `rules`"))
            })?;
            let path = base.join(rules);
            let yaml = std::fs::read_to_string(&path).map_err(|err| {
                AppError::CustomLang(format!("{name}: rules `{}`: {err}", path.display()))
            })?;
            // The grammar outlives the process (the dynamic registry never
            // unloads), so leaking its display name to `'static` is consistent
            // and lets it share `Language`'s built-in storage.
            let leaked: &'static str = Box::leak(name.clone().into_boxed_str());
            registry
                .langs
                .push(Language::from_rules(leaked, &yaml, Reformatter::None)?);
        }
        registry.apply_overrides(&config.languages, base)?;
        if config.no_reformat {
            registry.disable_reformat();
        } else {
            registry.apply_reformatters(&config.languages)?;
        }
        Ok(registry)
    }

    /// Detect a language from a path's file extension.
    #[must_use]
    pub fn detect(&self, path: &Path) -> Option<&Language> {
        let lang = <Lang as ast_grep_language::Language>::from_path(path)?;
        self.langs.iter().find(|registered| registered.lang == lang)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
