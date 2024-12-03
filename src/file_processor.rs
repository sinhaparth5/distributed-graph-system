use std::fs::File;
use std::io::{self, BufRead};
use serde::{Deserialize, Serialize};
use crate::graph::{Graph, Node, Edge};

#[derive(Debug, Serialize, Deserialize)]
pub enum FileFormat {
    EdgeList,
    AdjacencyList,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Invalid file format")]
    InvalidFormat,
    #[error("File I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Parsing error: {0}")]
    ParsingError(String),
}

pub fn process_file(path: &str, format: FileFormat) -> Result<Graph, ProcessError> {
    let mut graph = Graph::new();  // Use the new() constructor

    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    match format {
        FileFormat::EdgeList => {
            for line in reader.lines() {
                let line = line?;
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 || parts.len() > 3 {
                    return Err(ProcessError::ParsingError(
                        "EdgeList must have two nodes and optional weight per line".to_string()
                    ));
                }

                let u: usize = parts[0].parse().map_err(|_| 
                    ProcessError::ParsingError("Invalid node index".to_string())
                )?;
                let v: usize = parts[1].parse().map_err(|_| 
                    ProcessError::ParsingError("Invalid node index".to_string())
                )?;
                let weight = if parts.len() == 3 {
                    parts[2].parse().map_err(|_| 
                        ProcessError::ParsingError("Invalid weight".to_string())
                    )?
                } else {
                    1.0
                };

                let node_u = Node { id: u, data: format!("Node {}", u) };
                let node_v = Node { id: v, data: format!("Node {}", v) };

                graph.add_node(node_u);
                graph.add_node(node_v);
                graph.add_edge(Edge { from: u, to: v, weight });
            }
        },
        FileFormat::AdjacencyList => {
            for line in reader.lines() {
                let line = line?;
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() != 2 {
                    return Err(ProcessError::ParsingError(
                        "AdjacencyList must be in format 'node: neighbor1[,weight1], neighbor2[,weight2],...'".to_string()
                    ));
                }

                let u: usize = parts[0].trim().parse().map_err(|_| 
                    ProcessError::ParsingError("Invalid node index".to_string())
                )?;
                let node_u = Node { id: u, data: format!("Node {}", u) };
                graph.add_node(node_u);

                for neighbor_info in parts[1].split(',') {
                    let neighbor_parts: Vec<&str> = neighbor_info.trim().split_whitespace().collect();
                    if neighbor_parts.is_empty() {
                        continue;
                    }

                    let v: usize = neighbor_parts[0].parse().map_err(|_| 
                        ProcessError::ParsingError("Invalid neighbor index".to_string())
                    )?;
                    let weight = if neighbor_parts.len() > 1 {
                        neighbor_parts[1].parse().map_err(|_| 
                            ProcessError::ParsingError("Invalid weight".to_string())
                        )?
                    } else {
                        1.0
                    };

                    let node_v = Node { id: v, data: format!("Node {}", v) };
                    graph.add_node(node_v);
                    graph.add_edge(Edge { from: u, to: v, weight });
                }
            }
        },
    }

    Ok(graph)
}