use clap::Parser;

mod cli;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Process(args) => reddit_core::graph::run_process(args.into())?,
        Commands::Filter(args) => {
            reddit_core::filter::run_filter(args.into())?;
        }
    }

    Ok(())
}
