use anyhow::Result;
use indicatif::MultiProgress;

use crate::args::FilterArgs;

use super::utils;

/// Main entry point for the filter subcommand
pub fn run_filter(args: FilterArgs) -> Result<()> {
    // Validate first
    utils::validate_args(&args)?;

    let mb = MultiProgress::new();

    utils::run_filter(&args, mb)
}
