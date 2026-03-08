use std::fs::File;
use std::io::{self, BufRead};
use serde::{Deserialize, Serialize};
use crate::graph::{Graph, Node, Edge};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
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
    println!("Processing file: {}", path);
    let mut graph = Graph::new();

    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    match format {
        FileFormat::EdgeList => {
            for (line_number, line) in reader.lines().enumerate() {
                let line = line?;
                let line = line.trim_start_matches('\u{feff}').trim();

                // Skip blank lines and comment lines (# or %)
                if line.is_empty() || line.starts_with('#') || line.starts_with('%') {
                    continue;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 || parts.len() > 3 {
                    return Err(ProcessError::ParsingError(format!(
                        "Line {}: expected 'source dest [weight]', got {} token(s)",
                        line_number + 1,
                        parts.len()
                    )));
                }

                let u: usize = parts[0].parse().map_err(|e| {
                    ProcessError::ParsingError(format!(
                        "Line {}: invalid source node '{}': {}", line_number + 1, parts[0], e
                    ))
                })?;

                let v: usize = parts[1].parse().map_err(|e| {
                    ProcessError::ParsingError(format!(
                        "Line {}: invalid destination node '{}': {}", line_number + 1, parts[1], e
                    ))
                })?;

                // Weight is optional — default to 1.0 when not provided
                let weight: f64 = if parts.len() == 3 {
                    parts[2].parse().map_err(|e| {
                        ProcessError::ParsingError(format!(
                            "Line {}: invalid weight '{}': {}", line_number + 1, parts[2], e
                        ))
                    })?
                } else {
                    1.0
                };

                graph.add_node(Node { id: u, data: format!("Node {}", u) });
                graph.add_node(Node { id: v, data: format!("Node {}", v) });
                graph.add_edge(Edge { from: u, to: v, weight });
            }
        }

        FileFormat::AdjacencyList => {
            for (line_number, line) in reader.lines().enumerate() {
                let line = line?;
                let line = line.trim_start_matches('\u{feff}').trim();

                if line.is_empty() || line.starts_with('#') || line.starts_with('%') {
                    continue;
                }

                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() != 2 {
                    return Err(ProcessError::ParsingError(format!(
                        "Line {}: expected 'node: neighbor1,weight1 neighbor2,weight2...'",
                        line_number + 1
                    )));
                }

                let u: usize = parts[0].trim().parse().map_err(|e| {
                    ProcessError::ParsingError(format!(
                        "Line {}: invalid source node '{}': {}", line_number + 1, parts[0].trim(), e
                    ))
                })?;

                graph.add_node(Node { id: u, data: format!("Node {}", u) });

                for neighbor_info in parts[1].split_whitespace() {
                    let neighbor_parts: Vec<&str> = neighbor_info.split(',').collect();
                    if neighbor_parts.len() != 2 {
                        return Err(ProcessError::ParsingError(format!(
                            "Line {}: invalid neighbor '{}', expected 'node,weight'",
                            line_number + 1, neighbor_info
                        )));
                    }

                    let v: usize = neighbor_parts[0].parse().map_err(|e| {
                        ProcessError::ParsingError(format!(
                            "Line {}: invalid neighbor node '{}': {}", line_number + 1, neighbor_parts[0], e
                        ))
                    })?;

                    let weight: f64 = neighbor_parts[1].parse().map_err(|e| {
                        ProcessError::ParsingError(format!(
                            "Line {}: invalid weight '{}': {}", line_number + 1, neighbor_parts[1], e
                        ))
                    })?;

                    graph.add_node(Node { id: v, data: format!("Node {}", v) });
                    graph.add_edge(Edge { from: u, to: v, weight });
                }
            }
        }
    }

    println!("Loaded graph: {} nodes, {} edges", graph.node_count(), graph.edge_count());
    Ok(graph)
}
