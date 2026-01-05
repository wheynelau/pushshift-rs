use indicatif::{ProgressBar, ProgressStyle};
use petgraph::Graph;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;
use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

#[derive(Default)]
pub struct ThreadGraph {
    graph: Graph<(), ()>,
    node_map: HashMap<String, NodeIndex>,
    threads: Vec<NodeIndex>,
}

use super::models::Reddit;
use crate::common::writer::{JsonEntry, setup_writer};

impl ThreadGraph {
    pub fn new() -> Self {
        ThreadGraph {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            threads: Vec::new(),
        }
    }

    pub fn add_node(&mut self, id: &str) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(id) {
            idx
        } else {
            let idx = self.graph.add_node(());
            self.node_map.insert(id.to_string(), idx);
            idx
        }
    }

    pub fn add_edge(&mut self, from_id: &str, to_id: &str) {
        let from_idx = self.add_node(from_id);
        let to_idx = self.add_node(to_id);
        self.graph.add_edge(from_idx, to_idx, ());
    }

    pub fn tranverse(
        &self,
        mut vec_threads: Vec<Reddit>,
        output: PathBuf,
        level: i32,
    ) -> Result<()> {
        let mut writer = setup_writer(output, level);

        let pb = ProgressBar::new(self.graph.node_indices().len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
            )
            .unwrap()
            .progress_chars("##-"),
        );
        pb.set_message("Traversing...");

        for start in self.graph.node_indices() {
            let mut bfs = Dfs::new(&self.graph, start);

            let mut threads: Vec<usize> = Vec::new();

            while let Some(visited) = bfs.next(&self.graph) {
                threads.push(visited.index());
            }

            if threads.len() > 1 {
                let subreddit = vec_threads[threads[0]].subreddit.clone();
                let raw_content = threads
                    .iter_mut()
                    .map(|thread| std::mem::take(&mut vec_threads[*thread].selftext))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let length = raw_content.len();
                let entry = JsonEntry {
                    raw_content,
                    subreddit,
                    length,
                };
                let json_string = serde_json::to_string(&entry)?;
                writeln!(writer, "{json_string}")?;
            }
            pb.inc(1);
        }
        writer.flush()?;
        pb.finish_with_message("Completed!");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn show_threads(&self) {
        for node in self.graph.node_indices() {
            println!("{:?}", self.graph[node]);
        }
    }
    pub fn add_threads(&mut self, id: &str) {
        let idx = self.add_node(id);
        self.threads.push(idx);
    }
    pub fn is_in_map(&self, id: &str) -> bool {
        self.node_map.contains_key(id)
    }
}
