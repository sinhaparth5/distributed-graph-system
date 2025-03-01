use mpi::topology::Rank;
use mpi::traits::*;
use mpi::{self, request::WaitGuard};
use serde::{Deserialize, Serialize};

use crate::graph::{Graph, Edge, Node};
use std::collections::HashMap;

// Serializable message types for MPI communication
#[derive(Debug, Serialize, Deserialize)]
pub enum GraphTaskType {
    DFS { start_node: usize },
    BFS { start_node: usize },
    Dijkstra { start_node: usize },
    AStar { start_node: usize, goal_node: usize },
    BellmanFord { start_node: usize },
    Kruskal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphTask {
    pub task_type: GraphTaskType,
    pub graph_partition: GraphPartition,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphPartition {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub node_range: (usize, usize),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResult {
    pub path: Option<Vec<usize>>,
    pub distances: Option<Vec<f64>>,
    pub has_negative_cycle: Option<bool>,
}

impl GraphPartition {
    pub fn to_graph(&self) -> Graph {
        let mut graph = Graph::new();

        // Add all nodes to graph
        for node in &self.nodes {
            graph.add_node(node.clone());
        }

        for edge in &self.edges {
            graph.add_edge(edge.clone());
        }
        graph
    }
}

pub struct MPIProcessor {
    universe: mpi::environment::Universe,
    world: mpi::topology::SystemCommunicator,
    rank: Rank,
    size: Rank,
}

impl MPIProcessor {
    pub fn new() -> Self {
        let universe = mpi::initialize().unwrap();
        let world = universe.world();
        let rank = world.rank();
        let size = world.size();

        MPIProcessor {
            universe,
            world,
            rank,
            size,
        }
    }

    pub fn is_master(&self) -> bool {
        self.rank == 0
    }

    pub fn get_rank(&self) -> Rank {
        self.rank
    }

    pub fn get_size(&self) -> Rank {
        self.size
    }

    // Partition the graph for distributed processing
    pub fn partition_graph(&self, graph: &Graph) -> Vec<GraphPartition> {
        let node_ids: Vec<usize> = graph.get_all_node_ids();
        let total_nodes = node_ids.len();
        let nodes_per_process = (total_nodes + self.size as usize - 1)/ self.size as usize;
    }
}