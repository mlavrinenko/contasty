//! Per-language rule overrides: apply the `extend` / `override` keys of the
//! `[languages.<lang>]` entries on top of the built-in and dynamic rule sets,
//! after both are in place. Lives beside the registry so it can reach
//! `Language`'s private rule storage without widening its visibility.

use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use crate::AppError;
use crate::config::{LangConfig, RuleSource};

use super::dynamic::Lang;
use super::{Registry, RuleFile, compile_rule};

impl Registry {
    /// Apply the `extend` / `override` key of each `[languages.<lang>]` entry
    /// that sets one (entries with neither are skipped). `extend` appends the
    /// user file's compiled rules (so they run after the built-ins); `override`
    /// replaces the language's set outright. An override of a dynamic grammar
    /// swaps its declared rule file for the user file's.
    ///
    /// # Errors
    ///
    /// [`AppError::Config`] for an entry that sets both mode keys, an unknown
    /// language, an unreadable file, or a `language:` that disagrees with the
    /// table key; [`AppError::RuleParse`] / [`AppError::Rule`] for a malformed
    /// file or a rule that fails to compile against the grammar.
    pub(super) fn apply_overrides(
        &mut self,
        languages: &HashMap<String, LangConfig>,
        base: &Path,
    ) -> Result<(), AppError> {
        for (name, entry) in languages {
            let (path, replace) = match entry
                .rule_source()
                .map_err(|msg| AppError::Config(format!("languages.{name}: {msg}")))?
            {
                None => continue,
                Some(RuleSource::Extend(path)) => (path, false),
                Some(RuleSource::Override(path)) => (path, true),
            };
            let full = base.join(path);
            let target = self
                .langs
                .iter_mut()
                .find(|registered| registered.name == name.as_str())
                .ok_or_else(|| {
                    AppError::Config(format!(
                        "languages.{name}: unknown language (no rules registered for it)"
                    ))
                })?;
            let lang = target.lang;
            let yaml = std::fs::read_to_string(&full).map_err(|err| {
                AppError::Config(format!("languages.{name}: `{}`: {err}", full.display()))
            })?;
            let file: RuleFile = serde_yaml::from_str(&yaml)?;
            // `language:` is required by the schema; it must name the table key's
            // language so a misfiled rule set is a hard error, not a silent swap.
            if Lang::from_str(&file.language)? != lang {
                return Err(AppError::Config(format!(
                    "languages.{name}: file declares language `{}`, expected `{name}`",
                    file.language
                )));
            }
            let compiled = file
                .rules
                .into_iter()
                .map(|spec| compile_rule(lang, spec))
                .collect::<Result<Vec<_>, _>>()?;
            if replace {
                target.rules = compiled;
            } else {
                target.rules.extend(compiled);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "overrides_tests.rs"]
mod tests;
