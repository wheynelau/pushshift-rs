use clap::Parser;

pub mod cli;
pub mod common;
pub mod filter;
pub mod graph;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Process(args) => graph::run_process(args)?,
        Commands::Filter(args) => {
            filter::run_filter(args)?;
        }
    }

    Ok(())
}
