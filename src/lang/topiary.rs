//! Embedded Topiary reformatter backend (feature `topiary`).
//!
//! Topiary is tree-sitter based, like ast-grep, but ships its own grammar
//! stack and a per-language formatting query. We bundle the queries from
//! `topiary-queries` and the grammars we register; a language Topiary has no
//! query/grammar for is reported as unsupported (a config error upstream),
//! never a silent no-op. Like the shell-out backend, a runtime failure warns
//! and yields `None` so the caller keeps the unformatted splice.
//!
//! The set of supported languages is intentionally small: it overlaps the
//! languages contasty registers (today only Rust). Add a `query_for` +
//! `grammar_for` arm as more bundled grammars gain Topiary queries.

use topiary_core::{Language, Operation, TopiaryQuery, formatter_str};
use topiary_tree_sitter_facade::Language as TsLanguage;

/// True when both a Topiary query and a tree-sitter grammar are bundled for
/// `name`. Drives the upstream config-error vs. accept decision.
pub(super) fn supported(name: &str) -> bool {
    query_for(name).is_some() && grammar_for(name).is_some()
}

/// Reformat `source` for `name` via Topiary, or `None` (with a warning) on any
/// failure. Building the query per call keeps the backend stateless and
/// `Sync`-free across the parallel walk; reformatting is not the hot path.
pub(super) fn run(name: &str, source: &str) -> Option<String> {
    let language = build_language(name)?;
    let mut out: Vec<u8> = Vec::new();
    let operation = Operation::Format {
        skip_idempotence: true,
        tolerate_parsing_errors: false,
    };
    match formatter_str(source, &mut out, &language, operation) {
        Ok(()) => String::from_utf8(out).ok(),
        Err(err) => {
            log::warn!("reformat: topiary failed for `{name}`: {err}; keeping unformatted output");
            None
        }
    }
}

fn build_language(name: &str) -> Option<Language> {
    let grammar = grammar_for(name)?;
    let query = match TopiaryQuery::new(&grammar, query_for(name)?) {
        Ok(query) => query,
        Err(err) => {
            log::warn!("reformat: topiary query for `{name}` failed to compile: {err}");
            return None;
        }
    };
    Some(Language {
        name: name.to_owned(),
        query,
        grammar,
        indent: None,
    })
}

fn query_for(name: &str) -> Option<&'static str> {
    match name {
        "rust" => Some(topiary_queries::rust()),
        _ => None,
    }
}

fn grammar_for(name: &str) -> Option<TsLanguage> {
    match name {
        "rust" => Some(TsLanguage::from(tree_sitter_rust::LANGUAGE)),
        _ => None,
    }
}
