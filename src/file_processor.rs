use std::collections::HashMap;
use std::fs::File as StdFile;
use std::io::{self, BufRead};
use serde::{Deserialize, Serialize};
use crate::graph::{Graph, Node, Edge};

#[derive(Deserialize, Debug)]
pub enum FileFormat {
    EdgeList,
    AdjacencyList,
}

// Custom error type for file processing
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Invalid file format")]
    InvalidFormat,
    #[error("File I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Parsing error: {0}")]
    ParsingError(String),
}

// Function to process the graph file and return a graph
// Function to process the graph file and return a graph
pub fn process_file(path: &str, format: FileFormat) -> Result<Graph, ProcessError> {
    let mut graph = Graph {
        nodes: HashMap::new(),
        adj_list: HashMap::new(),
    };

    let file = StdFile::open(path).map_err(ProcessError::Io)?;
    let reader = io::BufReader::new(file);

    match format {
        FileFormat::EdgeList => {
            for line in reader.lines() {
                let line = line.map_err(ProcessError::Io)?;
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() != 2 {
                    return Err(ProcessError::ParsingError("EdgeList must have exactly two nodes per line".to_string()));
                }

                let u: usize = parts[0].parse().map_err(|_| ProcessError::ParsingError("Invalid node index".to_string()))?;
                let v: usize = parts[1].parse().map_err(|_| ProcessError::ParsingError("Invalid node index".to_string()))?;

                // Create Node instances with dummy data (adjust as needed)
                let node_u = Node { id: u, data: format!("Node {}", u) }; // You can modify the data as per your requirements
                let node_v = Node { id: v, data: format!("Node {}", v) };

                // Add nodes and edges to the graph
                graph.add_node(node_u);
                graph.add_node(node_v);
                let edge = Edge { from: u, to: v, weight: 1.0 }; // Edge weight can be set as needed
                graph.add_edge(edge);
            }
        },
        FileFormat::AdjacencyList => {
            for line in reader.lines() {
                let line = line.map_err(ProcessError::Io)?;
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() != 2 {
                    return Err(ProcessError::ParsingError("AdjacencyList must be in the format 'node: neighbor1, neighbor2,...'".to_string()));
                }

                let u: usize = parts[0].trim().parse().map_err(|_| ProcessError::ParsingError("Invalid node index".to_string()))?;
                let neighbors: Vec<usize> = parts[1].split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();

                // Create Node instance for the current node
                let node_u = Node { id: u, data: format!("Node {}", u) }; // You can modify the data as per your requirements
                graph.add_node(node_u); // Add the node to the graph

                for &v in &neighbors {
                    // Create Node instance for the neighbor
                    let node_v = Node { id: v, data: format!("Node {}", v) }; // You can modify the data as per your requirements
                    graph.add_node(node_v); // Add the neighbor to the graph

                    // Create and add the edge
                    let edge = Edge { from: u, to: v, weight: 1.0 }; // Edge weight can be set as needed
                    graph.add_edge(edge);
                }
            }
        },
    }

    Ok(graph)
}