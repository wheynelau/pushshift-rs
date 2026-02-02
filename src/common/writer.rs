use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::cli::CompressionArgs;

#[derive(Serialize, Deserialize)]
pub struct JsonEntry {
    pub raw_content: String,
    pub length: usize,
    pub subreddit: String,
    pub permalink: Option<String>,
    pub created_utc: Option<u64>,
}

pub fn setup_writer<P: AsRef<Path>>(
    filename: P,
    args: &CompressionArgs,
) -> Result<Box<dyn Write>, Error> {
    let path = filename.as_ref();
    let outfile = File::create(path)?;
    let writer: Box<dyn Write> = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
    {
        let mut encoder = zstd::stream::write::Encoder::new(outfile, args.level)?;

        encoder.multithread(args.workers)?;
        Box::new(encoder.auto_finish())
    } else {
        // Write directly for other files (e.g., .jsonl)
        Box::new(BufWriter::new(outfile))
    };
    Ok(writer)
}
