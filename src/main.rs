use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser, ValueEnum};

use contasty::StripSet;
use contasty::config::Config;
use contasty::inputs::IgnoreMode;

/// Output format for the stripped bundle.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    /// Line-numbered per-file dump: `N: <line>` with gaps for cuts (default).
    Lines,
    /// Markdown document with per-file fenced code blocks.
    Markdown,
    /// Pretty-printed JSON bundle: `{ base, files: [{ path, lang, content }] }`.
    Json,
}

/// Strips executable code from your source files, leaving declarations
/// behind — a tasty context bundle for your LLM.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Files, directories, or globs to process (repeatable). Each unfolds to a
    /// set of source files; the deduped union is stripped. Folders are walked
    /// `.gitignore`-aware; globs are expanded internally (quote them so the shell
    /// does not). Defaults to the current directory.
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Categories to strip (repeatable, interleaved with paths). Comma-separated:
    /// comments, imports, tests, body, all (alias: everything), none. Prefix a
    /// category with ! to remove it (e.g. all,!body). Each occurrence sets the
    /// strip set for the paths that follow until the next --strip.
    #[arg(long, value_name = "CATS")]
    strip: Vec<String>,

    /// Control .gitignore filtering (repeatable, interleaved with paths).
    #[arg(
        long,
        value_enum,
        value_name = "MODE",
        long_help = "Control .gitignore filtering (repeatable, interleaved with paths). \
Modes: enable (default, respect .gitignore), disable (include ignored files \
too), reverse (only .gitignored files). Each occurrence sets the mode for the \
paths that follow until the next --ignore.\n\
\n\
Examples:\n\
\x20 contasty --ignore=disable src/      include .gitignored files too\n\
\x20 contasty --ignore=reverse src/      only .gitignored files\n\
\x20 contasty A --ignore=disable B --ignore=enable C   per-path mode switching"
    )]
    ignore: Vec<IgnoreMode>,

    /// Print compactization statistics instead of the stripped code.
    #[arg(long)]
    stats: bool,

    /// Output format for the stripped bundle. Line-numbered by default.
    #[arg(long, value_enum, default_value = "lines")]
    format: OutputFormat,

    /// Path to a `contasty.toml` configuration file.
    #[arg(long)]
    config: Option<PathBuf>,
}

/// Group positional paths by interleaved `--ignore` and `--strip` switches.
///
/// Each path receives the most-recently-seen ignore mode (default `Enable`) and
/// CLI strip set. The strip is `None` until the first `--strip`, signalling
/// "no explicit selection — fall through to config layering"; an explicit
/// `--strip` sets `Some`. Consecutive paths sharing both are coalesced into one
/// group.
fn path_groups(
    m: &clap::ArgMatches,
) -> Result<Vec<(PathBuf, IgnoreMode, Option<StripSet>)>, String> {
    let mut events: Vec<(usize, Event)> = Vec::new();
    if let Some(modes) = m.get_many::<IgnoreMode>("ignore") {
        if let Some(indices) = m.indices_of("ignore") {
            for (mode_val, idx) in modes.copied().zip(indices) {
                events.push((idx, Event::Ignore(mode_val)));
            }
        }
    }
    if let Some(strips) = m.get_many::<String>("strip") {
        if let Some(indices) = m.indices_of("strip") {
            for (strip_val, idx) in strips.zip(indices) {
                let set = StripSet::parse_list(strip_val)?;
                events.push((idx, Event::Strip(set)));
            }
        }
    }
    if let Some(paths) = m.get_many::<PathBuf>("paths") {
        if let Some(indices) = m.indices_of("paths") {
            for (path, idx) in paths.cloned().zip(indices) {
                events.push((idx, Event::Path(path)));
            }
        }
    }
    events.sort_by_key(|&(idx, _)| idx);
    let mut out = Vec::new();
    let mut active_ignore = IgnoreMode::Enable;
    let mut active_strip: Option<StripSet> = None;
    for (_, event) in events {
        match event {
            Event::Ignore(mode_val) => active_ignore = mode_val,
            Event::Strip(set) => active_strip = Some(set),
            Event::Path(path) => out.push((path, active_ignore, active_strip)),
        }
    }
    Ok(out)
}

enum Event {
    Ignore(IgnoreMode),
    Strip(StripSet),
    Path(PathBuf),
}

fn main() -> Result<()> {
    env_logger::init();
    let m = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(&m)?;
    let cwd = std::env::current_dir()?;
    let config = Config::load(cli.config.as_deref(), &cwd);
    let groups = path_groups(&m).map_err(|msg| anyhow::anyhow!("{msg}"))?;
    let files = contasty::resolve(&groups, &cwd)?;
    let items = contasty::collect(&files, &config)?;
    if cli.stats {
        let report = contasty::stats::compute(&items);
        print!("{report}");
    } else {
        let rendered = match cli.format {
            OutputFormat::Lines => contasty::render_lines(&items),
            OutputFormat::Markdown => contasty::render_markdown(&items),
            OutputFormat::Json => contasty::render_json(&items),
        };
        std::io::stdout().write_all(rendered.as_bytes())?;
    }
    Ok(())
}
