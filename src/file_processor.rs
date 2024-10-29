use petgraph::graph::{Graph, NodeIndex};
use petgraph::prelude::*;
use std::fs::File as StdFile;
use std::io::{self, BufRead};
use std::path::Path;
use serde::{Deserialize, Serialize};

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
pub fn process_file(path: &str, format: FileFormat) -> Result<Graph<usize, usize>, ProcessError> {
    let mut graph = Graph::<usize, usize>::new();
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

                let u_index = graph.add_node(u);
                let v_index = graph.add_node(v);
                graph.add_edge(u_index, v_index, 1); // Edge weight can be set as needed
            }
        },
        FileFormat::AdjacencyList => {
            for line in reader.lines() {
                let line = line.map_err(ProcessError::Io)?;
                let parts: Vec<&str> = line.split(":").collect();
                if parts.len() != 2 {
                    return Err(ProcessError::ParsingError("AdjacencyList must be in the format 'node: neighbor1, neighbor2,...'".to_string()));
                }

                let u: usize = parts[0].trim().parse().map_err(|_| ProcessError::ParsingError("Invalid node index".to_string()))?;
                let neighbors: Vec<usize> = parts[1].split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();

                let u_index = graph.add_node(u);
                for &v in &neighbors {
                    let v_index = graph.add_node(v);
                    graph.add_edge(u_index, v_index, 1); // Edge weight can be set as needed
                }
            }
        },
    }

    Ok(graph)
}