use clap::{Args, Parser, Subcommand, ValueEnum};
use reddit_core::args;

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
/// BFS is not implemented yet
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum SearchMethod {
    BFS,
    DFS,
}

#[derive(Args, Clone)]
pub struct CompressionArgs {
    /// Compression level
    #[arg(
        short,
        long,
        help = "Compression level from 0-22, ignored if extension is not .zst",
        default_value = "3",
        value_parser = clap::value_parser!(i32).range(0..23)
    )]
    pub level: i32,

    #[arg(
        short,
        long,
        help = "Number of parallel worker threads for zstd compression (0 = single-threaded).",
        default_value = "0"
    )]
    pub workers: u32,
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

    /// Include score information in comment text
    #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
    pub include_scores: bool,

    #[command(flatten)]
    pub compression: CompressionArgs,
}

#[derive(Args, Clone)]
pub struct FilterArgs {
    /// Input file to filter, only .zst and .jsonl are supported, accepts a list of files
    #[arg(short, long, num_args = 1..)]
    pub input: Vec<std::path::PathBuf>,

    /// Output file template for filtered results
    /// Available placeholders: {basename} {subreddit} {timestamp}
    /// Extension is provided by user: only accepts .zst and .jsonl
    #[arg(
        short,
        long,
        help = "Output file template (e.g., {basename}_{subreddit}_filtered.jsonl)"
    )]
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
        help = "To filter multiple subreddits use -n subreddit 1 -n subreddit 2. Note that no check is done to ensure the subreddit exists.",
        value_parser = parse_lowercase,
        num_args = 1..,
        required(true)
    )]
    pub name: Vec<String>,

    #[command(flatten)]
    pub compression: CompressionArgs,
}

#[allow(clippy::unnecessary_wraps)]
fn parse_lowercase(s: &str) -> Result<std::string::String, anyhow::Error> {
    Ok(s.to_lowercase())
}

// Conversion impls from CLI types to core types

impl From<CompressionArgs> for args::CompressionArgs {
    fn from(cli: CompressionArgs) -> Self {
        Self {
            level: cli.level,
            workers: cli.workers,
        }
    }
}

impl From<ProcessArgs> for args::ProcessArgs {
    fn from(cli: ProcessArgs) -> Self {
        Self {
            submissions: cli.submissions,
            comments: cli.comments,
            output: cli.output,
            include_scores: cli.include_scores,
            compression: cli.compression.into(),
        }
    }
}

impl From<FilterArgs> for args::FilterArgs {
    fn from(cli: FilterArgs) -> Self {
        Self {
            input: cli.input,
            output: cli.output,
            split: cli.split,
            name: cli.name,
            compression: cli.compression.into(),
        }
    }
}
