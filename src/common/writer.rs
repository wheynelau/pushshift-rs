use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

pub struct JsonlWriter {
    writer: BufWriter<File>,
}
#[derive(Serialize, Deserialize)]
pub struct JsonEntry {
    pub raw_content: String,
    pub length: usize,
}

impl JsonlWriter {
    pub fn new(filename: PathBuf) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename)?;

        Ok(JsonlWriter {
            writer: BufWriter::new(file),
        })
    }

    pub fn write_line(&mut self, content: &JsonEntry) -> std::io::Result<()> {
        let json = serde_json::to_string(content)?;
        writeln!(self.writer, "{json}")?;
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub fn setup_writer<P: AsRef<Path>>(filename: P) -> Box<dyn Write> {
    let path = filename.as_ref();
    let outfile = File::create(path).expect("Failed to create output file");
    let writer: Box<dyn Write> = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
    {
        // Use zstd encoder for .zst files
        let encoder =
            zstd::stream::write::Encoder::new(outfile, 0).expect("Failed to create zstd encoder");
        Box::new(BufWriter::new(encoder))
    } else {
        // Write directly for other files (e.g., .jsonl)
        Box::new(BufWriter::new(outfile))
    };
    writer
}
