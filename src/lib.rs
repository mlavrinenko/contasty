//! contasty — strip executable code, keep declarations, render Markdown context.
//!
//! The library walks a path (gitignore-aware), detects each file's language from
//! its extension, parses with tree-sitter, elides bodies the language module
//! marks as "executable", and renders the result as a Markdown bundle suitable
//! for an LLM context window.

use std::path::PathBuf;

use thiserror::Error;

pub mod config;
mod lang;
mod render;
pub mod stats;
mod walk;

pub use lang::{Registry, rules_schema_json};
pub use render::{render_json, render_markdown};
pub use walk::{Stripped, collect};

/// Errors produced by the contasty library.
#[derive(Debug, Error)]
pub enum AppError {
    /// An I/O operation on a source file failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// The gitignore-aware walker reported an error.
    #[error("walk: {0}")]
    Walk(#[from] ignore::Error),

    /// An embedded rule set could not be parsed as YAML.
    #[error("rule parse: {0}")]
    RuleParse(#[from] serde_yaml::Error),

    /// A rule set referenced an unknown language or failed to compile against
    /// its grammar.
    #[error("rule: {0}")]
    Rule(String),

    /// Tree-sitter produced no tree (e.g. parser was cancelled).
    #[error("parse failed: {}", path.display())]
    ParseFailed {
        /// The file that failed to parse.
        path: PathBuf,
    },
}
