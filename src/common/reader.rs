use anyhow::{Error, bail};
use indicatif::ProgressBar;
/// Handles the reading of files
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use zstd::stream::read::Decoder;

pub fn setup_reader<P: AsRef<Path>>(
    input_path: P,
    pb: &ProgressBar,
) -> Result<Box<dyn BufRead>, Error> {
    let path = input_path.as_ref();
    let thread_file = File::open(path).expect("Failed to open input file");
    let progress_reader = pb.wrap_read(thread_file);
    let reader: Box<dyn BufRead> = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
    {
        // Use zstd decoder for .zst files
        let decoder = Decoder::new(progress_reader).expect("Failed to create zstd decoder");
        Box::new(BufReader::new(decoder))
    } else if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
    {
        // Read directly for other files (e.g., .jsonl)
        Box::new(BufReader::new(progress_reader))
    } else {
        bail!("Unsupported file type");
    };
    Ok(reader)
}
