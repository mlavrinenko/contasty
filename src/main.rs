use anyhow::Result;
use clap::Parser;

/// Strips all executable likes from your code to prepare tasty context for your agent.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Name to greet
    #[arg(default_value = "contasty")]
    name: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    contasty::greet(&cli.name)?;

    Ok(())
}
