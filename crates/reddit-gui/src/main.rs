#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use reddit_core::args::{CompressionArgs, FilterArgs, ProcessArgs};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuiCompressionArgs {
    level: i32,
    workers: u32,
}

impl From<GuiCompressionArgs> for CompressionArgs {
    fn from(g: GuiCompressionArgs) -> Self {
        CompressionArgs {
            level: g.level,
            workers: g.workers,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuiProcessArgs {
    submissions: Vec<String>,
    comments: Vec<String>,
    output: String,
    include_scores: bool,
    compression: GuiCompressionArgs,
}

impl From<GuiProcessArgs> for ProcessArgs {
    fn from(g: GuiProcessArgs) -> Self {
        ProcessArgs {
            submissions: g.submissions.into_iter().map(PathBuf::from).collect(),
            comments: g.comments.into_iter().map(PathBuf::from).collect(),
            output: PathBuf::from(g.output),
            include_scores: g.include_scores,
            compression: g.compression.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuiFilterArgs {
    input: Vec<String>,
    output: Option<String>,
    split: bool,
    name: Vec<String>,
    compression: GuiCompressionArgs,
}

impl From<GuiFilterArgs> for FilterArgs {
    fn from(g: GuiFilterArgs) -> Self {
        FilterArgs {
            input: g.input.into_iter().map(PathBuf::from).collect(),
            output: g.output.map(PathBuf::from),
            split: g.split,
            name: g.name,
            compression: g.compression.into(),
        }
    }
}

#[tauri::command]
fn run_process(args: GuiProcessArgs) -> Result<String, String> {
    reddit_core::graph::run_process(args.into()).map_err(|e| e.to_string())?;
    Ok("Process completed successfully".to_string())
}

#[tauri::command]
fn run_filter(args: GuiFilterArgs) -> Result<String, String> {
    reddit_core::filter::run_filter(args.into()).map_err(|e| e.to_string())?;
    Ok("Filter completed successfully".to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![run_process, run_filter])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
