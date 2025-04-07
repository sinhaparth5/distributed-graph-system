use std::fs::File;
use std::io::{self, BufRead};
use serde::{Deserialize, Serialize};
use crate::graph::{Graph, Node, Edge, NodeFeatures};
use std::collections::HashMap;
use std::path::Path;

// Update the FileFormat enum to include all file types
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")] 
pub enum FileFormat {
    EdgeList,       
    AdjacencyList,   
    Edges,           
    Circle,          
    Feat,            
    FeatNames,     
    EgoFeat,         
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
        },
        FileFormat::Edges => {
            // Process Facebook EDGES file (source destination pairs with implicit weight of 1.0)
            for (line_number, line) in reader.lines().enumerate() {
                let line = line?;
                
                // Skip empty lines
                if line.trim().is_empty() {
                    continue;
                }
                
                println!("Processing line {}: {}", line_number + 1, line);
                
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() != 2 {
                    return Err(ProcessError::ParsingError(
                        format!("Line {} must have exactly two values: source_node destination_node", line_number + 1)
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
                
                let weight: f64 = 1.0; // Use default weight of 1.0

                // Create and add nodes
                let node_u = Node { id: u, data: format!("Node {}", u) };
                let node_v = Node { id: v, data: format!("Node {}", v) };
                
                graph.add_node(node_u);
                graph.add_node(node_v);
                
                // Create and add edge
                let edge = Edge { from: u, to: v, weight };
                graph.add_edge(edge);
                
                // Also add the reverse edge to make it undirected
                let edge_rev = Edge { from: v, to: u, weight };
                graph.add_edge(edge_rev);
            }
        },
        FileFormat::Circle => {
            // Process Facebook CIRCLE file (circle_name node1 node2...)
            let mut circle_id = 1000000; // Start with a large ID to avoid collision with node IDs
            
            for (line_number, line) in reader.lines().enumerate() {
                let line = line?;
                
                // Skip empty lines
                if line.trim().is_empty() {
                    continue;
                }
                
                println!("Processing line {}: {}", line_number + 1, line);
                
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return Err(ProcessError::ParsingError(
                        format!("Line {} must have at least a circle name and one node ID", line_number + 1)
                    ));
                }

                let circle_name = parts[0].to_string();
                
                // Create a central node for the circle
                let circle_node = Node { 
                    id: circle_id, 
                    data: format!("Circle {}", circle_name) 
                };
                graph.add_node(circle_node);
                
                // Add edges from central circle node to all members
                for i in 1..parts.len() {
                    let node_id: usize = parts[i].parse().map_err(|e| {
                        ProcessError::ParsingError(
                            format!("Invalid node ID at line {}, position {}: {} - {}", 
                                   line_number + 1, i, parts[i], e)
                        )
                    })?;
                    
                    // Create the member node if it doesn't exist
                    if !graph.has_node(node_id) {
                        let node = Node { id: node_id, data: format!("Node {}", node_id) };
                        graph.add_node(node);
                    }
                    
                    // Connect circle to member
                    let edge = Edge { from: circle_id, to: node_id, weight: 1.0 };
                    graph.add_edge(edge);
                    
                    // Connect member to circle
                    let rev_edge = Edge { from: node_id, to: circle_id, weight: 1.0 };
                    graph.add_edge(rev_edge);
                    
                    // Connect all nodes in the same circle (optional - fully connected community)
                    for j in 1..i {
                        let other_id: usize = parts[j].parse().unwrap();
                        
                        // Add edges between members
                        let edge1 = Edge { from: node_id, to: other_id, weight: 0.5 };
                        let edge2 = Edge { from: other_id, to: node_id, weight: 0.5 };
                        
                        graph.add_edge(edge1);
                        graph.add_edge(edge2);
                    }
                }
                
                circle_id += 1;
            }
        },
        FileFormat::Feat => {
            // Process Facebook FEAT file (node_id feature_values...)
            let mut features_map = HashMap::new();
            
            for (line_number, line) in reader.lines().enumerate() {
                let line = line?;
                
                // Skip empty lines
                if line.trim().is_empty() {
                    continue;
                }
                
                println!("Processing line {}: {}", line_number + 1, line);
                
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return Err(ProcessError::ParsingError(
                        format!("Line {} must have a node ID followed by feature values", line_number + 1)
                    ));
                }

                let node_id: usize = parts[0].parse().map_err(|e| {
                    ProcessError::ParsingError(
                        format!("Invalid node ID at line {}: {} - {}", line_number + 1, parts[0], e)
                    )
                })?;
                
                // Extract feature values (convert strings to numbers)
                let mut feature_values = Vec::new();
                for i in 1..parts.len() {
                    let val: f64 = parts[i].parse().map_err(|e| {
                        ProcessError::ParsingError(
                            format!("Invalid feature value at line {}, position {}: {} - {}", 
                                   line_number + 1, i, parts[i], e)
                        )
                    })?;
                    feature_values.push(val);
                }
                
                // Store features for this node
                features_map.insert(node_id, feature_values);
                
                // Create the node if it doesn't exist
                if !graph.has_node(node_id) {
                    let node = Node { id: node_id, data: format!("Node {}", node_id) };
                    graph.add_node(node);
                }
            }
            
            // Add features to the graph
            graph.set_node_features(NodeFeatures::VectorPerNode(features_map));
        },
        FileFormat::FeatNames => {
            // Process Facebook FEATNAMES file (feature_id feature_description)
            let mut feature_descriptions = HashMap::new();
            
            for (line_number, line) in reader.lines().enumerate() {
                let line = line?;
                
                // Skip empty lines
                if line.trim().is_empty() {
                    continue;
                }
                
                println!("Processing line {}: {}", line_number + 1, line);
                
                // Split by first space character
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() != 2 {
                    return Err(ProcessError::ParsingError(
                        format!("Line {} must have a feature ID followed by a description", line_number + 1)
                    ));
                }

                let feature_id: usize = parts[0].parse().map_err(|e| {
                    ProcessError::ParsingError(
                        format!("Invalid feature ID at line {}: {} - {}", line_number + 1, parts[0], e)
                    )
                })?;
                
                let description = parts[1].to_string();
                feature_descriptions.insert(feature_id, description);
            }
            
            // Store feature descriptions in the graph
            graph.set_feature_descriptions(feature_descriptions);
        },
        FileFormat::EgoFeat => {
            // Process Facebook EGOFEAT file (binary feature vector for the ego node)
            // We'll use node ID 0 as the ego node
            let ego_node_id = 0;
            
            for (line_number, line) in reader.lines().enumerate() {
                let line = line?;
                
                // Skip empty lines
                if line.trim().is_empty() {
                    continue;
                }
                
                println!("Processing line {}: {}", line_number + 1, line);
                
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    return Err(ProcessError::ParsingError(
                        format!("Line {} must contain binary feature values", line_number + 1)
                    ));
                }

                // Extract feature values (convert strings to numbers)
                let mut feature_values = Vec::new();
                for (i, part) in parts.iter().enumerate() {
                    let val: f64 = part.parse().map_err(|e| {
                        ProcessError::ParsingError(
                            format!("Invalid feature value at line {}, position {}: {} - {}", 
                                   line_number + 1, i, part, e)
                        )
                    })?;
                    feature_values.push(val);
                }
                
                // Store ego node features
                let mut features_map = HashMap::new();
                features_map.insert(ego_node_id, feature_values);
                
                // Add ego features to the graph
                graph.set_ego_features(features_map);
                
                // Create the ego node if it doesn't exist
                if !graph.has_node(ego_node_id) {
                    let node = Node { id: ego_node_id, data: "Ego Node".to_string() };
                    graph.add_node(node);
                }
                
                // Only process the first line
                break;
            }
        }
    }

    Ok(graph)
}

// Helper function to process all Facebook file types for an ego network
pub fn process_facebook_ego_network(ego_id: usize) -> Result<Graph, ProcessError> {
    // Base directory for Facebook data - adjust path as needed
    let base_dir = ".";
    
    // Initialize the graph
    let mut graph = Graph::new();
    
    // Filenames
    let edges_file = format!("{}/{}.edges", base_dir, ego_id);
    let circles_file = format!("{}/{}.circles", base_dir, ego_id);
    let feat_file = format!("{}/{}.feat", base_dir, ego_id);
    let featnames_file = format!("{}/{}.featnames", base_dir, ego_id);
    let egofeat_file = format!("{}/{}.egofeat", base_dir, ego_id);
    
    // Process each file if it exists
    if Path::new(&edges_file).exists() {
        let edges_graph = process_file(&edges_file, FileFormat::Edges)?;
        graph.merge(edges_graph);
    }
    
    if Path::new(&circles_file).exists() {
        let circles_graph = process_file(&circles_file, FileFormat::Circle)?;
        graph.merge(circles_graph);
    }
    
    if Path::new(&feat_file).exists() {
        let feat_graph = process_file(&feat_file, FileFormat::Feat)?;
        graph.merge(feat_graph);
    }
    
    if Path::new(&featnames_file).exists() {
        let featnames_graph = process_file(&featnames_file, FileFormat::FeatNames)?;
        graph.merge(featnames_graph);
    }
    
    if Path::new(&egofeat_file).exists() {
        let egofeat_graph = process_file(&egofeat_file, FileFormat::EgoFeat)?;
        graph.merge(egofeat_graph);
    }
    
    Ok(graph)
}