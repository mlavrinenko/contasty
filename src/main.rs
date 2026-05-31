use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};

use contasty::config::Config;

/// Output format for the stripped bundle.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    /// Markdown document with per-file fenced code blocks (default).
    Markdown,
    /// Pretty-printed JSON bundle: `{ base, files: [{ path, lang, content }] }`.
    Json,
}

/// Strips executable code from your source files, leaving declarations
/// behind — a tasty context bundle for your LLM.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Directory or file to process. Walks `.gitignore`-aware.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Keep `#[cfg(test)]` modules and `#[test]` functions in the output.
    /// Off by default — test code is noise for most context-bundle use cases.
    #[arg(long)]
    include_tests: bool,

    /// Keep every comment (including doc comments) in the output.
    /// Off by default — comments are noise for most context-bundle use cases.
    #[arg(long)]
    include_comments: bool,

    /// Print compactization statistics instead of the stripped code.
    /// Shows original vs compacted line counts (code, comments, blanks).
    #[arg(long)]
    stats: bool,

    /// Output format for the stripped bundle. Markdown by default.
    #[arg(long, value_enum, default_value = "markdown")]
    format: OutputFormat,

    /// Path to a `contasty.toml` configuration file.
    /// When not set, defaults to `contasty.toml` in the current directory.
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let cwd = std::env::current_dir()?;
    let config = Config::load(cli.config.as_deref(), &cwd);
    let items = contasty::collect(
        &cli.path,
        !cli.include_tests,
        !cli.include_comments,
        &config.compact,
    )?;
    if cli.stats {
        let report = contasty::stats::compute(&items);
        print!("{report}");
    } else {
        let rendered = match cli.format {
            OutputFormat::Markdown => contasty::render_markdown(&items),
            OutputFormat::Json => contasty::render_json(&items),
        };
        std::io::stdout().write_all(rendered.as_bytes())?;
    }
    Ok(())
}
