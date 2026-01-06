use indicatif::{ProgressBar, ProgressStyle};
use petgraph::Direction;
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

    pub fn tranverse(&self, vec_threads: Vec<Reddit>, output: PathBuf, level: i32) -> Result<()> {
        let mut writer = setup_writer(output, level);

        let pb = ProgressBar::new(self.threads.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
            )
            .unwrap()
            .progress_chars("##-"),
        );
        pb.set_message("Traversing...");

        for start in &self.threads {
            // Find all leaf nodes in this thread's subgraph
            let mut leaves: Vec<NodeIndex> = Vec::new();
            let mut dfs = Dfs::new(&self.graph, *start);

            while let Some(node) = dfs.next(&self.graph) {
                // A leaf has no outgoing edges
                if self
                    .graph
                    .neighbors_directed(node, Direction::Outgoing)
                    .next()
                    .is_none()
                {
                    leaves.push(node);
                }
            }

            // For each leaf, backtrack to root and create a linear path
            for leaf in leaves {
                let mut path: Vec<NodeIndex> = Vec::new();
                let mut current = Some(leaf);

                // Backtrack following incoming edges to root
                while let Some(node) = current {
                    path.push(node);
                    // Move to parent via incoming edge
                    current = self
                        .graph
                        .neighbors_directed(node, Direction::Incoming)
                        .next();
                }

                // Reverse to get root -> ... -> leaf order
                path.reverse();

                // Only process paths with more than just the root
                if path.len() > 1 {
                    let subreddit = vec_threads[path[0].index()].subreddit.clone();
                    let permalink = vec_threads[path[0].index()].permalink.clone();

                    let raw_content = path
                        .iter()
                        .enumerate()
                        .map(|(i, node)| {
                            let content = &vec_threads[node.index()].selftext;
                            if i == 0 {
                                format!("# Post\n\n{content}")
                            } else {
                                format!("\n\n---\n\n## Reply\n\n{content}")
                            }
                        })
                        .collect::<Vec<_>>()
                        .concat();

                    let length = raw_content.len();
                    let entry = JsonEntry {
                        raw_content,
                        subreddit,
                        length,
                        permalink,
                    };
                    let json_string = serde_json::to_string(&entry)?;
                    writeln!(writer, "{json_string}")?;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_path_extraction_two_branches() {
        let mut graph = ThreadGraph::new();
        let mut vec_threads: Vec<Reddit> = Vec::new();

        // Build tree: A -> B -> C and A -> D -> E
        graph.add_node("A");
        graph.add_node("B");
        graph.add_node("C");
        graph.add_node("D");
        graph.add_node("E");

        graph.add_edge("A", "B");
        graph.add_edge("B", "C");
        graph.add_edge("A", "D");
        graph.add_edge("D", "E");

        graph.add_threads("A");

        vec_threads.push(Reddit {
            id: "A".to_string(),
            selftext: "Content A".to_string(),
            subreddit: "test".to_string(),
            parent_id: None,
            permalink: Some("/r/test/A".to_string()),
        });
        vec_threads.push(Reddit {
            id: "B".to_string(),
            selftext: "Content B".to_string(),
            subreddit: "test".to_string(),
            parent_id: Some("A".to_string()),
            permalink: Some("/r/test/B".to_string()),
        });
        vec_threads.push(Reddit {
            id: "C".to_string(),
            selftext: "Content C".to_string(),
            subreddit: "test".to_string(),
            parent_id: Some("B".to_string()),
            permalink: Some("/r/test/C".to_string()),
        });
        vec_threads.push(Reddit {
            id: "D".to_string(),
            selftext: "Content D".to_string(),
            subreddit: "test".to_string(),
            parent_id: Some("A".to_string()),
            permalink: Some("/r/test/D".to_string()),
        });
        vec_threads.push(Reddit {
            id: "E".to_string(),
            selftext: "Content E".to_string(),
            subreddit: "test".to_string(),
            parent_id: Some("D".to_string()),
            permalink: Some("/r/test/E".to_string()),
        });

        // Traverse using linear path extraction
        let mut outputs = Vec::new();

        for start in &graph.threads {
            // Find all leaf nodes
            let mut leaves: Vec<NodeIndex> = Vec::new();
            let mut dfs = Dfs::new(&graph.graph, *start);

            while let Some(node) = dfs.next(&graph.graph) {
                if graph
                    .graph
                    .neighbors_directed(node, Direction::Outgoing)
                    .next()
                    .is_none()
                {
                    leaves.push(node);
                }
            }

            // For each leaf, backtrack to root
            for leaf in leaves {
                let mut path: Vec<NodeIndex> = Vec::new();
                let mut current = Some(leaf);

                while let Some(node) = current {
                    path.push(node);
                    current = graph
                        .graph
                        .neighbors_directed(node, Direction::Incoming)
                        .next();
                }

                path.reverse();

                if path.len() > 1 {
                    let raw_content = path
                        .iter()
                        .enumerate()
                        .map(|(i, node)| {
                            let content = &vec_threads[node.index()].selftext;
                            if i == 0 {
                                format!("# Post\n\n{content}")
                            } else {
                                format!("\n\n---\n\n## Reply\n\n{content}")
                            }
                        })
                        .collect::<Vec<_>>()
                        .concat();
                    outputs.push((start.index(), path, raw_content));
                }
            }
        }

        // Verify: Should have 2 paths (A->B->C and A->D->E)
        assert_eq!(outputs.len(), 2);

        // Verify each path contains the correct content and markdown structure
        let contents: Vec<_> = outputs.iter().map(|(_, _, c)| c.clone()).collect();

        // Verify markdown headers are present
        for content in &contents {
            assert!(content.contains("# Post"), "Missing # Post header");
            assert!(content.contains("## Reply"), "Missing ## Reply header");
            assert!(content.contains("---"), "Missing separator");
        }

        // Path A->B->C
        let path_abc = contents.iter().find(|c| {
            c.contains("Content A")
                && c.contains("Content B")
                && c.contains("Content C")
                && !c.contains("Content D")
                && !c.contains("Content E")
        });
        assert!(path_abc.is_some(), "Path A->B->C not found");

        // Path A->D->E
        let path_ade = contents.iter().find(|c| {
            c.contains("Content A")
                && c.contains("Content D")
                && c.contains("Content E")
                && !c.contains("Content B")
                && !c.contains("Content C")
        });
        assert!(path_ade.is_some(), "Path A->D->E not found");
    }
}
