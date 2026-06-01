use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser, ValueEnum};

use contasty::CategorySelection;
use contasty::config::Config;

/// Output format for the stripped bundle.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    /// Markdown document with per-file fenced code blocks (default).
    Markdown,
    /// Pretty-printed JSON bundle: `{ base, files: [{ path, lang, content }] }`.
    Json,
}

/// A category of code elements to include or exclude.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Selector {
    /// Comment lines and blocks (including doc comments).
    Comments,
    /// Import / use declarations.
    Imports,
    /// Test functions and test modules.
    Tests,
    /// All three categories (alias: everything).
    #[value(alias = "everything")]
    All,
}

/// Strips executable code from your source files, leaving declarations
/// behind — a tasty context bundle for your LLM.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Directory or file to process. Walks `.gitignore`-aware.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Include a category in the output (repeatable). Selectors: comments,
    /// imports, tests, all (alias: everything). Applied left-to-right with
    /// --exclude; last mention of a category wins.
    #[arg(long, value_enum, value_name = "SEL")]
    include: Vec<Selector>,

    /// Exclude a category from the output (repeatable). Selectors: comments,
    /// imports, tests, all (alias: everything). Applied left-to-right with
    /// --include; last mention of a category wins.
    #[arg(long, value_enum, value_name = "SEL")]
    exclude: Vec<Selector>,

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

#[derive(Clone, Copy)]
enum Op {
    Include,
    Exclude,
}

fn ordered_selectors(m: &clap::ArgMatches) -> Vec<(Op, Selector)> {
    let mut events: Vec<(usize, Op, Selector)> = Vec::new();
    for (id, op) in [("include", Op::Include), ("exclude", Op::Exclude)] {
        let Some(values) = m.get_many::<Selector>(id) else {
            continue;
        };
        let Some(indices) = m.indices_of(id) else {
            continue;
        };
        for (sel, idx) in values.copied().zip(indices) {
            events.push((idx, op, sel));
        }
    }
    events.sort_by_key(|&(idx, _, _)| idx);
    events.into_iter().map(|(_, op, sel)| (op, sel)).collect()
}

fn cli_override(ops: &[(Op, Selector)]) -> CategorySelection {
    let mut sel = CategorySelection::default();
    for &(op, selector) in ops {
        let on = matches!(op, Op::Include);
        match selector {
            Selector::Comments => sel.comments = Some(on),
            Selector::Imports => sel.imports = Some(on),
            Selector::Tests => sel.tests = Some(on),
            Selector::All => {
                sel.comments = Some(on);
                sel.imports = Some(on);
                sel.tests = Some(on);
            }
        }
    }
    sel
}

fn main() -> Result<()> {
    env_logger::init();
    let m = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(&m)?;
    let cwd = std::env::current_dir()?;
    let config = Config::load(cli.config.as_deref(), &cwd);
    let override_sel = cli_override(&ordered_selectors(&m));
    let items = contasty::collect(&cli.path, override_sel, &config)?;
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
