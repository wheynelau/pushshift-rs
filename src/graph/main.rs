use super::models::Reddit;
use crate::{
    cli::ProcessArgs,
    graph::models::{Comment, Thread},
};
use indicatif::{ProgressBar, ProgressStyle};
// use rayon::prelude::*;
use std::io::BufRead;

use anyhow::Result;

use crate::common::setup_reader;
pub fn run_process(args: ProcessArgs) -> Result<()> {
    let mut thread_graph = super::threadgraph::ThreadGraph::new();

    let pb = ProgressBar::no_length();
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bytes} ({bytes_per_sec}) {msg}")
            .expect("Failed to set progress style"),
    );
    let mut submission_count = 0u32;
    for path in args.submissions {
        pb.set_message(format!("Processing submission file: {path:?}"));
        let reader = setup_reader(path, &pb)?;
        reader
            .lines()
            .try_for_each::<_, Result<(), std::io::Error>>(|line| {
                let line = line?;
                if let Ok(json) = serde_json::from_str::<Thread>(&line)
                    && let Ok(reddit) = TryInto::<Reddit>::try_into(json)
                {
                    let id = reddit.id.clone();
                    thread_graph.add_reddit_data(&id, reddit);
                    thread_graph.add_threads(&id);
                }
                Ok(())
            })?;
        submission_count += 1;
    }
    pb.finish_with_message(format!("Completed {submission_count} submissions file"));

    let pb = ProgressBar::no_length();
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bytes} ({bytes_per_sec}) {msg}")
            .expect("Failed to set progress style"),
    );
    submission_count = 0;
    for path in args.comments {
        pb.set_message(format!("Processing comments file {path:?}"));
        let reader = setup_reader(path, &pb)?;
        reader
            .lines()
            .try_for_each::<_, Result<(), std::io::Error>>(|line| {
                let line = line?;
                if let Ok(json) = serde_json::from_str::<Comment>(&line)
                    && let Some(comment) = json.into_reddit(args.include_scores)
                    && let Some(parent_id) = comment.parent_id.clone()
                    && thread_graph.is_in_map(&parent_id)
                {
                    let id = comment.id.clone();
                    thread_graph.add_reddit_data(&id, comment);
                    thread_graph.add_edge(&parent_id, &id);
                }
                Ok(())
            })?;
        submission_count += 1;
    }
    pb.finish_with_message(format!("Completed {submission_count} comments file"));

    thread_graph.traverse(args.output, &args.compression)?;
    Ok(())
}
