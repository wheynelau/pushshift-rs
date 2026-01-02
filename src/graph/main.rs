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
    let mut threads: Vec<Reddit> = Vec::new();

    let pb = ProgressBar::no_length();
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bytes} ({bytes_per_sec}) {msg}")
            .expect("Failed to set progress style"),
    );
    let mut submission_count = 0u32;
    for path in args.submissions {
        let reader = setup_reader(path, &pb);
        reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str::<Thread>(&line).ok())
            .filter_map(|json| TryInto::<Reddit>::try_into(json).ok())
            .for_each(|thread| {
                thread_graph.add_threads(&thread.id);
                thread_graph.add_node(&thread.id);
                threads.push(thread);
            });
        submission_count += 1;
    }
    pb.finish_with_message(format!("Completed {submission_count} submissions file"));

    let pb = ProgressBar::no_length();
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] {bytes} ({bytes_per_sec}) {msg}")
            .expect("Failed to set progress style"),
    );
    // Test out if loading all to memory is a good idea
    submission_count = 0;
    args.comments.into_iter().for_each(|path| {
        let reader = setup_reader(path, &pb);
        reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str::<Comment>(&line).ok())
            .filter_map(|json| json.into_reddit(args.include_scores).ok())
            .for_each(|comment| {
                if let Some(parent_id) = &comment.parent_id
                    && thread_graph.is_in_map(parent_id)
                {
                    thread_graph.add_node(&comment.id);
                    thread_graph.add_edge(parent_id, &comment.id);
                    threads.push(comment);
                }
            });
        submission_count += 1;
    });
    pb.finish_with_message(format!("Completed {submission_count} comments file"));

    thread_graph.tranverse(threads, args.output, args.compression.level)?;
    Ok(())
}
