use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

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
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let items = contasty::collect(&cli.path, !cli.include_tests)?;
    let md = contasty::render_markdown(&items);
    std::io::stdout().write_all(md.as_bytes())?;
    Ok(())
}
