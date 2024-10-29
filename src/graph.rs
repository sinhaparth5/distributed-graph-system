use serde::{ Deserialize, Serialize };
use std::collections::{HashSet, VecDeque, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: usize,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

pub struct Graph {
    nodes: HashMap<usize, Node>,
    adj_list: HashMap<usize, Vec<(usize, f64)>>
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: HashMap::new(),
            adj_list: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id, node.clone());
        self.adj_list.entry(node.id).or_insert(Vec::new());
    }
}