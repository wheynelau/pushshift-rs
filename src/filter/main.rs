use anyhow::Result;
use indicatif::MultiProgress;

use crate::cli::FilterArgs;

use super::utils;

/// Main entry point for the filter subcommand
pub fn run_filter(args: FilterArgs) -> Result<()> {
    // Validate first
    utils::validate_args(&args)?;

    let mb = MultiProgress::new();

    if args.multithread {
        utils::run_filter_mt(&args, mb)
    } else {
        utils::run_filter_st(&args, mb)
    }
}
