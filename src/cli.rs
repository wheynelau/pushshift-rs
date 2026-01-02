use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "reddit-rs")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Process Reddit threads and comments
    Process(ProcessArgs),
    /// Filter Reddit data based on criteria
    Filter(FilterArgs),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum SearchMethod {
    BFS,
    DFS,
}

#[derive(Args, Clone)]
pub struct ProcessArgs {
    // Input files, for now only jsonl is supported
    #[arg(short, long, num_args = 1..)]
    pub submissions: Vec<std::path::PathBuf>,

    #[arg(short, long, num_args = 1..)]
    pub comments: Vec<std::path::PathBuf>,

    #[arg(short, long)]
    pub output: std::path::PathBuf,
}

#[derive(Args, Clone)]
pub struct FilterArgs {
    /// Input file to filter, only .zst and .jsonl are supported, accepts a list of files
    #[arg(short, long, num_args = 1..)]
    pub input: Vec<std::path::PathBuf>,

    /// Output file template for filtered results
    /// Available placeholders: {basename} (input filename without extension),
    /// {subreddit} (required when using --split), {timestamp} (Unix timestamp)
    /// Examples:
    ///   filtered_{subreddit}.jsonl           → `filtered_singapore.jsonl`
    ///   outputs/{basename}_{subreddit}.jsonl → outputs/RC_2025-09_singapore.jsonl
    ///   data/{basename}_{timestamp}.jsonl    → data/RC_2025-09_1735830645.jsonl
    ///   data/{basename}_{timestamp}.zst -> data/RC_2025-09_1735830645.zst
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,

    /// Split into multiple files by subreddits
    #[arg(
        short,
        long,
        help = "Split into multiple files by subreddits. Requires --output to contain {{subreddit}} placeholder"
    )]
    pub split: bool,

    /// Filter subreddits by name
    #[arg(
        short,
        long,
        help = "To filter multiple subreddits use -n subreddit 1 -n subreddit 2",
        value_parser = parse_lowercase,
        num_args = 0..
    )]
    pub name: Vec<String>,

    /// Compression level
    #[arg(
        short,
        long,
        help = "Compression level from 1-19, ignored if extension is not .zst",
        default_value = "3"
    )]
    pub level: u32,

    /// Run in multithreaded mode
    #[arg(short,
        long,
        action=clap::ArgAction::SetTrue,
        default_value = "false", 
        help = "Multithreaded may be misleading here, but what it means is a single thread is used for read and decompress, a second thread is for json and writes
                This can be useful if you have a fast IO.")]
    pub multithread: bool,
}

#[allow(clippy::unnecessary_wraps)]
fn parse_lowercase(s: &str) -> Result<std::string::String, anyhow::Error> {
    Ok(s.to_lowercase())
}
