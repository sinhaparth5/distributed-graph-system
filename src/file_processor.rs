use std::fs::File;
use std::io::{self, BufRead};
use serde::{Deserialize, Serialize};
use crate::graph::{Graph, Node, Edge};

// Add both Serialize and Deserialize derives
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]  // This helps with JSON compatibility
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
                println!("Processing line {}: {}", line_number + 1, line);
                
                // Trim BOM if present and other whitespace
                let line = line.trim_start_matches('\u{feff}').trim();
                
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() != 3 {
                    return Err(ProcessError::ParsingError(
                        format!("Line {} must have exactly three values: source_node destination_node weight", line_number + 1)
                    ));
                }

                let u: usize = parts[0].parse().map_err(|e| {
                    ProcessError::ParsingError(
                        format!("Invalid source node at line {}: {} - {}", line_number + 1, parts[0], e)
                    )
                })?;
                
                let v: usize = parts[1].parse().map_err(|e| {
                    ProcessError::ParsingError(
                        format!("Invalid destination node at line {}: {} - {}", line_number + 1, parts[1], e)
                    )
                })?;
                
                let weight: f64 = parts[2].parse().map_err(|e| {
                    ProcessError::ParsingError(
                        format!("Invalid weight at line {}: {} - {}", line_number + 1, parts[2], e)
                    )
                })?;

                println!("Adding nodes {} and {} with weight {}", u, v, weight);

                // Create and add nodes
                let node_u = Node { id: u, data: format!("Node {}", u) };
                let node_v = Node { id: v, data: format!("Node {}", v) };
                
                graph.add_node(node_u);
                graph.add_node(node_v);
                
                // Create and add edge
                let edge = Edge { from: u, to: v, weight };
                graph.add_edge(edge);
            }
        },
        FileFormat::AdjacencyList => {
            for (line_number, line) in reader.lines().enumerate() {
                let line = line?;
                println!("Processing line {}: {}", line_number + 1, line);
                
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() != 2 {
                    return Err(ProcessError::ParsingError(
                        format!("Line {} must be in format 'node: neighbor1,weight1 neighbor2,weight2...'", line_number + 1)
                    ));
                }

                let u: usize = parts[0].trim().parse().map_err(|e| {
                    ProcessError::ParsingError(
                        format!("Invalid source node at line {}: {} - {}", line_number + 1, parts[0], e)
                    )
                })?;

                let node_u = Node { id: u, data: format!("Node {}", u) };
                graph.add_node(node_u);

                for neighbor_info in parts[1].split_whitespace() {
                    let neighbor_parts: Vec<&str> = neighbor_info.split(',').collect();
                    if neighbor_parts.len() != 2 {
                        return Err(ProcessError::ParsingError(
                            format!("Invalid neighbor format at line {}: {}", line_number + 1, neighbor_info)
                        ));
                    }

                    let v: usize = neighbor_parts[0].parse().map_err(|e| {
                        ProcessError::ParsingError(
                            format!("Invalid neighbor node at line {}: {} - {}", line_number + 1, neighbor_parts[0], e)
                        )
                    })?;

                    let weight: f64 = neighbor_parts[1].parse().map_err(|e| {
                        ProcessError::ParsingError(
                            format!("Invalid weight at line {}: {} - {}", line_number + 1, neighbor_parts[1], e)
                        )
                    })?;

                    let node_v = Node { id: v, data: format!("Node {}", v) };
                    graph.add_node(node_v);
                    graph.add_edge(Edge { from: u, to: v, weight });
                }
            }
        }
    }

    Ok(graph)
}