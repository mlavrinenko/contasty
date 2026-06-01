//! Per-language post-strip reformatter: the generalized successor to the old
//! `Option<fn(&str) -> Option<String>>` formatter slot.
//!
//! After splicing, each language runs its [`Reformatter`] over the stripped
//! text. Rust keeps its built-in prettyplease pass; every other language is
//! `None` by default and opts in via the `reformat` key of its
//! `[languages.<lang>]` config entry — either the embedded Topiary backend
//! (feature `topiary`) or a shell-out command. A reformat failure is never
//! fatal: it warns and falls back to the unformatted splice.

use std::collections::HashMap;

use crate::AppError;
use crate::config::{LangConfig, Reformat, ReformatMode};

use super::Registry;
use super::shellout;

/// How a registered language reformats its stripped output.
pub(super) enum Reformatter {
    /// Keep the raw splice verbatim.
    None,
    /// A compiled-in formatter (e.g. Rust's prettyplease). Only runs when
    /// comments are dropped: `syn`-based formatters discard non-doc comments, so
    /// formatting under `--include-comments` would lose them. Returns `None`
    /// when it cannot parse the post-strip source.
    Builtin(fn(&str) -> Option<String>),
    /// Shell out to an external formatter (argv vector, stdin -> stdout).
    Command(Vec<String>),
    /// Embedded Topiary backend keyed by the language's name.
    #[cfg(feature = "topiary")]
    Topiary(&'static str),
}

impl Reformatter {
    /// Reformat `source`, falling back to it unchanged on any failure. The
    /// `drop_comments` flag gates only the comment-lossy [`Self::Builtin`] pass.
    pub(super) fn apply(&self, source: &str, drop_comments: bool) -> String {
        match self {
            Self::None => source.to_owned(),
            Self::Builtin(formatter) => run_builtin(*formatter, source, drop_comments),
            Self::Command(argv) => shellout::run(argv, source).unwrap_or_else(|| source.to_owned()),
            #[cfg(feature = "topiary")]
            Self::Topiary(name) => {
                super::topiary::run(name, source).unwrap_or_else(|| source.to_owned())
            }
        }
    }
}

fn run_builtin(formatter: fn(&str) -> Option<String>, source: &str, drop_comments: bool) -> String {
    if !drop_comments {
        return source.to_owned();
    }
    formatter(source).unwrap_or_else(|| source.to_owned())
}

impl Registry {
    /// Apply the `reformat` key of every `[languages.<lang>]` entry that sets
    /// one, replacing the language's default reformatter. Entries without
    /// `reformat` keep their built-in default (Rust: prettyplease; others: none).
    ///
    /// # Errors
    ///
    /// [`AppError::Config`] when `reformat` names a language with no registered
    /// rules, the command vector is empty, or `"topiary"` is requested for a
    /// language with no Topiary query (or without the `topiary` build feature).
    pub(super) fn apply_reformatters(
        &mut self,
        languages: &HashMap<String, LangConfig>,
    ) -> Result<(), AppError> {
        for (name, cfg) in languages {
            let Some(reformat) = &cfg.reformat else {
                continue;
            };
            let target = self
                .langs
                .iter_mut()
                .find(|registered| registered.name == name.as_str())
                .ok_or_else(|| {
                    AppError::Config(format!(
                        "languages.{name}: reformat set for unknown language (no rules registered)"
                    ))
                })?;
            let lang_name = target.name;
            target.reformat = build_reformatter(lang_name, reformat)?;
        }
        Ok(())
    }

    /// Force every registered language to skip reformatting (the `--no-reformat`
    /// kill-switch), including the built-in Rust prettyplease pass.
    pub(super) fn disable_reformat(&mut self) {
        for lang in &mut self.langs {
            lang.reformat = Reformatter::None;
        }
    }
}

/// Resolve a config [`Reformat`] for a registered language into a concrete
/// [`Reformatter`], surfacing unsupported selections as a clear config error.
fn build_reformatter(name: &'static str, reformat: &Reformat) -> Result<Reformatter, AppError> {
    match reformat {
        Reformat::Mode(ReformatMode::None) => Ok(Reformatter::None),
        Reformat::Command { command } => {
            if command.is_empty() {
                return Err(AppError::Config(format!(
                    "languages.{name}: reformat command is empty"
                )));
            }
            Ok(Reformatter::Command(command.clone()))
        }
        Reformat::Mode(ReformatMode::Topiary) => topiary_reformatter(name),
    }
}

#[cfg(feature = "topiary")]
fn topiary_reformatter(name: &'static str) -> Result<Reformatter, AppError> {
    if super::topiary::supported(name) {
        Ok(Reformatter::Topiary(name))
    } else {
        Err(AppError::Config(format!(
            "languages.{name}: embedded Topiary has no formatting query for `{name}`"
        )))
    }
}

#[cfg(not(feature = "topiary"))]
fn topiary_reformatter(name: &str) -> Result<Reformatter, AppError> {
    Err(AppError::Config(format!(
        "languages.{name}: reformat = \"topiary\" needs contasty built with --features topiary"
    )))
}
