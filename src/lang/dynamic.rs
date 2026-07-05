//! Dynamic (user-supplied) tree-sitter grammars via `ast-grep-dynamic`.
//!
//! The 28 grammars `ast-grep` bundles cover the common case with zero `.so`.
//! For anything it does not ship, a user drops a compiled grammar plus a rule
//! file, registers it under `[languages.<lang>]` with a `libraryPath`, and
//! contasty strips matching files with no rebuild.
//!
//! [`Lang`] unifies a built-in [`SupportLang`] and a registered [`DynamicLang`]
//! behind one type the rest of the engine speaks, so the parse/match/splice
//! pipeline stays language-agnostic.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use ast_grep_core::matcher::{Pattern, PatternBuilder, PatternError};
use ast_grep_core::tree_sitter::{StrDoc, TSLanguage};
use ast_grep_dynamic::{CustomLang, DynamicLang, LibraryPath};
use ast_grep_language::{Language, LanguageExt, SupportLang};

use crate::AppError;
use crate::config::{self, LangConfig};

/// A language the engine can strip: an `ast-grep` built-in or a dynamically
/// loaded grammar. Both halves are `Copy` handles into process-global tables,
/// so this stays cheap to pass by value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// One of the grammars `ast-grep` bundles.
    Builtin(SupportLang),
    /// A grammar loaded from a user-supplied shared library.
    Dynamic(DynamicLang),
}

impl Language for Lang {
    fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let path = path.as_ref();
        // Dynamic first: a custom grammar may deliberately claim an extension a
        // built-in also handles (override), mirroring `ast-grep`'s own order.
        DynamicLang::from_path(path)
            .map(Self::Dynamic)
            .or_else(|| SupportLang::from_path(path).map(Self::Builtin))
    }

    fn pre_process_pattern<'q>(&self, query: &'q str) -> Cow<'q, str> {
        match self {
            Self::Builtin(lang) => lang.pre_process_pattern(query),
            Self::Dynamic(lang) => lang.pre_process_pattern(query),
        }
    }

    fn meta_var_char(&self) -> char {
        match self {
            Self::Builtin(lang) => lang.meta_var_char(),
            Self::Dynamic(lang) => lang.meta_var_char(),
        }
    }

    fn expando_char(&self) -> char {
        match self {
            Self::Builtin(lang) => lang.expando_char(),
            Self::Dynamic(lang) => lang.expando_char(),
        }
    }

    fn kind_to_id(&self, kind: &str) -> u16 {
        match self {
            Self::Builtin(lang) => lang.kind_to_id(kind),
            Self::Dynamic(lang) => lang.kind_to_id(kind),
        }
    }

    fn field_to_id(&self, field: &str) -> Option<u16> {
        match self {
            Self::Builtin(lang) => lang.field_to_id(field),
            Self::Dynamic(lang) => lang.field_to_id(field),
        }
    }

    fn build_pattern(&self, builder: &PatternBuilder) -> Result<Pattern, PatternError> {
        builder.build(|src| StrDoc::try_new(src, *self))
    }
}

impl LanguageExt for Lang {
    fn get_ts_language(&self) -> TSLanguage {
        match self {
            Self::Builtin(lang) => lang.get_ts_language(),
            Self::Dynamic(lang) => lang.get_ts_language(),
        }
    }
}

impl FromStr for Lang {
    type Err = AppError;

    /// Resolve a `language:` name: a built-in first, then an already-registered
    /// dynamic grammar. Dynamic names resolve only after [`register`] has run.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        if let Ok(lang) = SupportLang::from_str(name) {
            Ok(Self::Builtin(lang))
        } else if let Ok(lang) = DynamicLang::from_str(name) {
            Ok(Self::Dynamic(lang))
        } else {
            Err(AppError::Rule(format!("unknown language `{name}`")))
        }
    }
}

/// Register every configured custom grammar with `ast-grep`'s process-global
/// `DynamicLang` table. Idempotent: names already registered are skipped, so
/// repeated calls (e.g. a library consumer invoking `collect` twice) are safe.
///
/// Every `library_path` is already absolute — resolved by
/// [`crate::config::Config::load`] against its own defining config file's
/// directory (project or global) — before this ever runs, so a neutral base
/// (`.`) satisfies `CustomLang::register`'s join with no effect. The registry
/// is intentionally leak-on-purpose in `ast-grep` (dropping a `Library` nulls
/// its symbols), so grammars register once and are never unloaded.
///
/// # Errors
///
/// [`AppError::CustomLang`] if a library is missing, exposes the wrong symbol,
/// or was built for an incompatible tree-sitter / target — surfaced as an
/// actionable message rather than a panic.
pub fn register(languages: &HashMap<String, LangConfig>) -> Result<(), AppError> {
    let langs: HashMap<String, CustomLang> = languages
        .iter()
        .filter(|(name, cfg)| cfg.is_dynamic() && DynamicLang::from_str(name).is_err())
        .map(|(name, cfg)| to_custom_lang(name, cfg).map(|lang| (name.clone(), lang)))
        .collect::<Result<_, AppError>>()?;
    if langs.is_empty() {
        return Ok(());
    }
    CustomLang::register(Path::new("."), langs).map_err(|err| AppError::CustomLang(err.to_string()))
}

/// Lower a custom-grammar [`LangConfig`] onto the `ast_grep_dynamic::CustomLang`
/// the registry consumes, dropping the contasty-only rule fields. Called only
/// for entries with `library_path` set; a missing `extensions` list is a hard
/// error since a grammar with no claimed extensions can never match a file.
fn to_custom_lang(name: &str, cfg: &LangConfig) -> Result<CustomLang, AppError> {
    let library_path = cfg
        .library_path
        .as_ref()
        .expect("register filters to dynamic entries");
    if cfg.extensions.is_empty() {
        return Err(AppError::CustomLang(format!(
            "languages.{name}: custom grammar needs a non-empty `extensions` list"
        )));
    }
    Ok(CustomLang {
        library_path: match library_path {
            config::LibraryPath::Single(path) => LibraryPath::Single(path.clone()),
            config::LibraryPath::Platform(map) => LibraryPath::Platform(map.clone()),
        },
        language_symbol: cfg.language_symbol.clone(),
        meta_var_char: cfg.meta_var_char,
        expando_char: cfg.expando_char,
        extensions: cfg.extensions.clone(),
    })
}
