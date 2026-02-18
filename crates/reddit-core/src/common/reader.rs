use anyhow::{Context, Error, bail};
use indicatif::ProgressBar;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Handles the reading of files
pub fn setup_reader<P: AsRef<Path>>(
    input_path: P,
    pb: &ProgressBar,
) -> Result<Box<dyn BufRead>, Error> {
    let path = input_path.as_ref();
    let file =
        File::open(path).context(format!("Failed to open input file: {}", path.display()))?;
    let progress_reader = pb.wrap_read(file);

    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
    {
        let mut decoder = zstd::stream::read::Decoder::new(progress_reader)
            .context("Failed to create zstd decoder")?;
        decoder.window_log_max(31)?;
        Ok(Box::new(BufReader::new(decoder)))
    } else if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
    {
        Ok(Box::new(BufReader::new(progress_reader)))
    } else {
        bail!(
            "Unsupported file type: {}",
            path.extension().unwrap_or_default().to_string_lossy()
        );
    }
}
