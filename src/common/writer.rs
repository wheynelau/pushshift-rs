use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct JsonEntry {
    pub raw_content: String,
    pub length: usize,
    pub subreddit: String,
    pub permalink: Option<String>,
    pub created_utc: Option<u64>,
}

pub fn setup_writer<P: AsRef<Path>>(filename: P, level: i32) -> Box<dyn Write> {
    let path = filename.as_ref();
    let outfile = File::create(path).expect("Failed to create output file");
    let writer: Box<dyn Write> = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
    {
        let encoder = zstd::stream::write::Encoder::new(outfile, level)
            .expect("Failed to create zstd encoder")
            .auto_finish();
        Box::new(encoder)
    } else {
        // Write directly for other files (e.g., .jsonl)
        Box::new(BufWriter::new(outfile))
    };
    writer
}
