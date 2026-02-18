use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct CompressionArgs {
    /// Compression level from 0-22, ignored if extension is not .zst
    pub level: i32,
    /// Number of parallel worker threads for zstd compression (0 = single-threaded)
    pub workers: u32,
}

impl Default for CompressionArgs {
    fn default() -> Self {
        Self {
            level: 3,
            workers: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcessArgs {
    pub submissions: Vec<PathBuf>,
    pub comments: Vec<PathBuf>,
    pub output: PathBuf,
    pub include_scores: bool,
    pub compression: CompressionArgs,
}

#[derive(Clone, Debug)]
pub struct FilterArgs {
    pub input: Vec<PathBuf>,
    pub output: Option<PathBuf>,
    pub split: bool,
    pub name: Vec<String>,
    pub compression: CompressionArgs,
}
