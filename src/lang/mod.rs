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
use ast_grep_language::SupportLang;
use serde::Deserialize;

use crate::AppError;
use crate::config::CompactConfig;

mod rust;

type Doc = StrDoc<SupportLang>;
type AstNode<'r> = ast_grep_core::Node<'r, Doc>;

/// A registered language: a tree-sitter grammar plus its compiled rule set.
pub struct Language {
    /// Markdown fence info-string (e.g. `"rust"`).
    pub name: &'static str,
    lang: SupportLang,
    rules: Vec<CompiledRule>,
    /// Optional source formatter applied after splicing. Returns `None` if it
    /// cannot handle the (post-strip) source — caller keeps the unformatted
    /// output rather than failing the whole file.
    format: Option<fn(&str) -> Option<String>>,
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
struct StripOptions<'cfg> {
    drop_tests: bool,
    drop_comments: bool,
    drop_imports: bool,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleFile {
    language: String,
    rules: Vec<RuleSpec>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
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

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "kebab-case")]
enum Gate {
    #[default]
    Always,
    Tests,
    Comments,
    Imports,
}

impl Gate {
    const fn enabled(self, opts: &StripOptions) -> bool {
        match self {
            Self::Always => true,
            Self::Tests => opts.drop_tests,
            Self::Comments => opts.drop_comments,
            Self::Imports => opts.drop_imports,
        }
    }
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
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

impl Language {
    /// Compile a language descriptor from an embedded ast-grep rule set.
    ///
    /// # Errors
    ///
    /// [`AppError::RuleParse`] if the YAML is malformed, [`AppError::Rule`] if
    /// the declared language is unknown or any rule fails to compile against the
    /// grammar.
    fn from_rules(
        name: &'static str,
        yaml: &str,
        format: Option<fn(&str) -> Option<String>>,
    ) -> Result<Self, AppError> {
        let file: RuleFile = serde_yaml::from_str(yaml)?;
        let lang = SupportLang::from_str(&file.language)
            .map_err(|_| AppError::Rule(format!("unknown language `{}`", file.language)))?;
        let rules = file
            .rules
            .into_iter()
            .map(|spec| compile_rule(lang, spec))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            name,
            lang,
            rules,
            format,
        })
    }

    /// Strip elidable nodes from `source`. The `drop_*` flags gate the test,
    /// comment, and import rules respectively.
    ///
    /// # Errors
    ///
    /// [`AppError::ParseFailed`] if tree-sitter cannot produce a parse tree.
    #[allow(clippy::too_many_arguments)]
    pub fn strip(
        &self,
        source: &str,
        path: &Path,
        drop_tests: bool,
        drop_comments: bool,
        drop_imports: bool,
        compact: &CompactConfig,
    ) -> Result<String, AppError> {
        let grep = AstGrep::try_new(source, self.lang).map_err(|_| AppError::ParseFailed {
            path: path.to_path_buf(),
        })?;
        let opts = StripOptions {
            drop_tests,
            drop_comments,
            drop_imports,
            compact,
        };
        let ranges = self.collect(&grep, &opts);
        let spliced = splice(source, &ranges);
        // Only format when comments are dropped: `syn`-based formatters discard
        // non-doc comments, so formatting under --include-comments would lose them.
        if !drop_comments {
            return Ok(spliced);
        }
        Ok(self
            .format
            .and_then(|formatter| formatter(&spliced))
            .unwrap_or(spliced))
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

fn compile_rule(lang: SupportLang, spec: RuleSpec) -> Result<CompiledRule, AppError> {
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
        Ok(Self {
            langs: vec![Language::from_rules(
                "rust",
                rust::RULES,
                Some(rust::format),
            )?],
        })
    }

    /// Detect a language from a path's file extension.
    #[must_use]
    pub fn detect(&self, path: &Path) -> Option<&Language> {
        let lang = <SupportLang as ast_grep_core::Language>::from_path(path)?;
        self.langs.iter().find(|registered| registered.lang == lang)
    }
}

const ELISION: &str = "{}";
const STR_TRUNCATION: &str = "\"[…CTY]\"";

fn splice(source: &str, ranges: &[(usize, usize, Action)]) -> String {
    if ranges.is_empty() {
        return source.to_owned();
    }
    let sorted = sort_ranges(ranges);
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0_usize;
    for &(start, end, action) in &sorted {
        if start < cursor {
            continue;
        }
        out.push_str(source.get(cursor..start).unwrap_or_default());
        cursor = apply(action, &mut out, source, end);
    }
    out.push_str(source.get(cursor..).unwrap_or_default());
    out
}

fn apply(action: Action, out: &mut String, source: &str, end: usize) -> usize {
    match action {
        Action::Elide => {
            out.push_str(ELISION);
            end
        }
        Action::Delete => consume_trailing_newline(source, end),
        Action::TruncateString => {
            out.push_str(STR_TRUNCATION);
            end
        }
    }
}

fn consume_trailing_newline(source: &str, end: usize) -> usize {
    if source.as_bytes().get(end) == Some(&b'\n') {
        end + 1
    } else {
        end
    }
}

fn sort_ranges(ranges: &[(usize, usize, Action)]) -> Vec<(usize, usize, Action)> {
    let mut sorted: Vec<_> = ranges.to_vec();
    // Sort by start ascending, then by end descending so a wider range that
    // shares a start wins over a narrower one (the narrower is skipped via
    // `start < cursor`).
    sorted.sort_by(|left, right| left.0.cmp(&right.0).then(right.1.cmp(&left.1)));
    sorted.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);
    sorted
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
