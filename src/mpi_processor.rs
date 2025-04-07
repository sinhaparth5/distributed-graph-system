use mpi::point_to_point::{Source, Status};
use mpi::topology::Rank;
use mpi::traits::*;
use mpi::{self, request::WaitGuard};
use serde::{Deserialize, Serialize};
use bincode::{serialize, deserialize};
use mpi::environment::Threading;

use crate::graph::{Graph, Edge, Node, NodeFeatures};
use std::collections::{HashMap, HashSet};

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
pub struct TaskResult {
    pub path: Option<Vec<usize>>,
    pub distances: Option<Vec<f64>>,
    pub has_negative_cycle: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphPartition {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub node_range: (usize, usize),
    pub node_features: HashMap<usize, Vec<f64>>,
    pub feature_descriptions: HashMap<usize, String>,
    pub ego_features: HashMap<usize, Vec<f64>>,
}

impl GraphPartition {
    // Create a new empty partition
    pub fn new() -> Self {
        GraphPartition {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_range: (0, 0),
            node_features: HashMap::new(),
            feature_descriptions: HashMap::new(),
            ego_features: HashMap::new(),
        }
    }

    pub fn to_graph(&self) -> Graph {
        let mut graph = Graph::new();

        // Add all nodes to graph
        for node in &self.nodes {
            graph.add_node(node.clone());
        }

        // Add all edges
        for edge in &self.edges {
            graph.add_edge(edge.clone());
        }
        
        // Add node features if any
        if !self.node_features.is_empty() {
            graph.set_node_features(NodeFeatures::VectorPerNode(self.node_features.clone()));
        }
        
        // Add feature descriptions if any
        if !self.feature_descriptions.is_empty() {
            graph.set_feature_descriptions(self.feature_descriptions.clone());
        }
        
        // Add ego features if any
        if !self.ego_features.is_empty() {
            graph.set_ego_features(self.ego_features.clone());
        }
        
        graph
    }
}
// Internal enum to represent our execution mode
enum ExecutionMode {
    Distributed {
        universe: mpi::environment::Universe,
        world: mpi::topology::SimpleCommunicator,
        rank: Rank,
        size: Rank,
    },
    SingleProcess,
}

pub struct MPIProcessor {
    mode: ExecutionMode,
}

impl MPIProcessor {
    pub fn new() -> Self {
        // Try to initialize MPI with thread support first, then fall back
        println!("Initializing MPI processor...");
        
        match mpi::initialize_with_threading(Threading::Multiple) {
            Some((universe, provided_threading)) => {
                let world = universe.world();
                let rank = world.rank();
                let size = world.size();
                
                println!("MPI initialized successfully: {} processes with thread support level: {:?}", size, provided_threading);
                
                MPIProcessor {
                    mode: ExecutionMode::Distributed {
                        universe,
                        world,
                        rank,
                        size,
                    },
                }
            },
            None => {
                // MPI initialization with thread support failed - try without thread support
                println!("WARNING: MPI initialization with thread support failed");
                println!("Trying without thread support...");
                
                match mpi::initialize() {
                    Some(universe) => {
                        let world = universe.world();
                        let rank = world.rank();
                        let size = world.size();
                        
                        println!("MPI initialized successfully: {} processes", size);
                        
                        MPIProcessor {
                            mode: ExecutionMode::Distributed {
                                universe,
                                world,
                                rank,
                                size,
                            },
                        }
                    },
                    None => {
                        // MPI initialization failed - create a single-process environment
                        println!("WARNING: MPI initialization failed. Running in single-process mode!");
                        println!("To debug MPI issues, run 'cargo run --bin mpi_test'");
                        
                        MPIProcessor {
                            mode: ExecutionMode::SingleProcess,
                        }
                    }
                }
            }
        }
    }
    
    pub fn is_master(&self) -> bool {
        match &self.mode {
            ExecutionMode::Distributed { rank, .. } => *rank == 0,
            ExecutionMode::SingleProcess => true,
        }
    }

    pub fn get_rank(&self) -> Rank {
        match &self.mode {
            ExecutionMode::Distributed { rank, .. } => *rank,
            ExecutionMode::SingleProcess => 0,
        }
    }

    pub fn get_size(&self) -> Rank {
        match &self.mode {
            ExecutionMode::Distributed { size, .. } => *size,
            ExecutionMode::SingleProcess => 1,
        }
    }

    // Partition the graph for distributed processing
    pub fn partition_graph(&self, graph: &Graph) -> Vec<GraphPartition> {
        // Get all node IDs from the graph
        let node_ids: Vec<usize> = graph.get_nodes().keys().cloned().collect();
        let total_nodes = node_ids.len();
        
        // Initialize empty collections for features
        let mut all_node_features = HashMap::new();
        let mut feature_descriptions = HashMap::new();
        let mut ego_features = HashMap::new();
        
        // Extract node features for all nodes
        for &node_id in &node_ids {
            if let Some(features) = graph.get_node_features(node_id) {
                all_node_features.insert(node_id, features);
            }
        }
        
        // Collect feature descriptions - These would typically come from get_feature_description(id)
        for feature_id in 0..1000 { // Use a reasonable upper bound or determine dynamically
            if let Some(desc) = graph.get_feature_description(feature_id) {
                feature_descriptions.insert(feature_id, desc.clone());
            }
        }
        
        // Collect ego features - typically for node 0 or specified ego nodes
        for ego_id in &[0] { // Adjust based on your data model
            if let Some(features) = graph.get_ego_features(*ego_id) {
                ego_features.insert(*ego_id, features.clone());
            }
        }
        
        // For single process mode, just return one partition with all nodes and features
        if let ExecutionMode::SingleProcess = self.mode {
            let all_node_ids: HashSet<usize> = node_ids.iter().cloned().collect();
            let all_nodes: Vec<Node> = all_node_ids
                .iter()
                .filter_map(|&id| graph.get_node(id).cloned())
                .collect();
            let all_edges = self.get_edges_for_partition(graph, &all_node_ids);
            
            return vec![GraphPartition {
                nodes: all_nodes,
                edges: all_edges,
                node_range: (0, total_nodes),
                node_features: all_node_features,
                feature_descriptions,
                ego_features,
            }];
        }
        
        // For distributed mode, partition by process
        let size = self.get_size();
        let nodes_per_process = (total_nodes + size as usize - 1) / size as usize;
        
        let mut partitions = Vec::with_capacity(size as usize);
        
        // Create partitions
        for proc_idx in 0..size {
            let start_idx = proc_idx as usize * nodes_per_process;
            let end_idx = std::cmp::min((proc_idx as usize + 1) * nodes_per_process, total_nodes);
            
            // Skip if this partition would be empty
            if start_idx >= total_nodes {
                continue;
            }
            
            // Get node IDs for this partition
            let partition_node_ids: HashSet<usize> = node_ids[start_idx..end_idx].iter().cloned().collect();
            
            // Create nodes for this partition
            let partition_nodes: Vec<Node> = partition_node_ids
                .iter()
                .filter_map(|&id| graph.get_node(id).cloned())
                .collect();
                
            // Get all edges where at least one endpoint is in this partition
            let partition_edges: Vec<Edge> = self.get_edges_for_partition(graph, &partition_node_ids);
            
            // Extract features for nodes in this partition
            let mut partition_features = HashMap::new();
            for &node_id in &partition_node_ids {
                if let Some(features) = all_node_features.get(&node_id) {
                    partition_features.insert(node_id, features.clone());
                }
            }
            
            // Create the partition
            partitions.push(GraphPartition {
                nodes: partition_nodes,
                edges: partition_edges,
                node_range: (start_idx, end_idx),
                node_features: partition_features,
                feature_descriptions: feature_descriptions.clone(),
                ego_features: ego_features.clone(),
            });
        }
        
        partitions
    }
    
    // Get all edges where at least one endpoint is in the partition
    fn get_edges_for_partition(&self, graph: &Graph, partition_node_ids: &HashSet<usize>) -> Vec<Edge> {
        let mut edges = Vec::new();
        
        for &node_id in partition_node_ids {
            if let Some(neighbors) = graph.get_neighbors(node_id) {
                for &(neighbor_id, weight) in neighbors {
                    // Create an edge for each neighbor
                    edges.push(Edge {
                        from: node_id,
                        to: neighbor_id,
                        weight,
                    });
                }
            }
        }
        
        edges
    }
    
    // Execute a distributed graph algorithm
    pub fn execute_distributed_algorithm(&self, graph: &Graph, task_type: GraphTaskType) -> TaskResult {
        println!("Process {} of {} executing algorithm", self.get_rank(), self.get_size());
        
        match &self.mode {
            ExecutionMode::Distributed { .. } => {
                if self.is_master() {
                    // Master process handles partitioning and distribution
                    self.execute_master(graph, task_type)
                } else {
                    // Worker processes handle computation
                    self.execute_worker()
                }
            },
            ExecutionMode::SingleProcess => {
                // In single process mode, just process the whole graph directly
                println!("Running in single-process mode");
                let partitions = self.partition_graph(graph);
                if !partitions.is_empty() {
                    self.process_task(&GraphTask {
                        task_type,
                        graph_partition: partitions[0].clone(),
                    })
                } else {
                    TaskResult {
                        path: Some(Vec::new()),
                        distances: Some(Vec::new()),
                        has_negative_cycle: None,
                    }
                }
            }
        }
    }
    
    // Master process: partition graph, distribute work, collect results
    fn execute_master(&self, graph: &Graph, task_type: GraphTaskType) -> TaskResult {
        // Only call this in distributed mode
        if let ExecutionMode::Distributed { world, .. } = &self.mode {
            // Partition the graph
            let partitions = self.partition_graph(graph);
            
            println!("Master process partitioning graph into {} parts", partitions.len());
            for (i, partition) in partitions.iter().enumerate() {
                println!("Partition {} has {} nodes and {} edges", 
                        i, partition.nodes.len(), partition.edges.len());
            }
            
            // Send tasks to worker processes
            for (i, partition) in partitions.iter().enumerate().skip(1) {
                if i < self.get_size() as usize {
                    let task = GraphTask {
                        task_type: self.clone_task_type(&task_type),
                        graph_partition: partition.clone(),
                    };
                    
                    // Serialize the task
                    let serialized_task = serialize(&task).expect("Failed to serialize task");
                    
                    // Send to worker process
                    world.process_at_rank(i as Rank)
                        .send(&serialized_task[..]);
                }
            }
            
            // Process the first partition in the master process
            let mut master_result = if !partitions.is_empty() {
                self.process_task(&GraphTask {
                    task_type: self.clone_task_type(&task_type),
                    graph_partition: partitions[0].clone(),
                })
            } else {
                TaskResult {
                    path: Some(Vec::new()),
                    distances: Some(Vec::new()),
                    has_negative_cycle: None,
                }
            };
            
            // Collect results from workers
            for i in 1..self.get_size() {
                if i as usize >= partitions.len() {
                    continue;
                }
                
                // Receive result from worker
                let (result_data, _) = world.process_at_rank(i)
                    .receive_vec::<u8>();
                
                // Deserialize the result
                let worker_result: TaskResult = deserialize(&result_data).expect("Failed to deserialize result");
                
                // Merge results
                self.merge_results(&mut master_result, &worker_result);
            }
            
            master_result
        } else {
            panic!("execute_master called in single-process mode");
        }
    }
    
    // Worker process: receive task, compute, send back result
    fn execute_worker(&self) -> TaskResult {
        // Only call this in distributed mode
        if let ExecutionMode::Distributed { world, .. } = &self.mode {
            // Receive task from master
            let (task_data, _): (Vec<u8>, Status) = world.process_at_rank(0)
                .receive_vec::<u8>();
            
            // Deserialize the task
            let task: GraphTask = deserialize(&task_data).expect("Failed to deserialize task");
            
            println!("Worker process {} received task with {} nodes and {} edges", 
                    self.get_rank(), task.graph_partition.nodes.len(), task.graph_partition.edges.len());
            
            // Process the task
            let result = self.process_task(&task);
            
            // Serialize the result
            let serialized_result = serialize(&result).expect("Failed to serialize result");
            
            // Send result back to master
            world.process_at_rank(0)
                .send(&serialized_result[..]);
            
            result
        } else {
            panic!("execute_worker called in single-process mode");
        }
    }
    
    // Process a single graph task
    fn process_task(&self, task: &GraphTask) -> TaskResult {
        println!("Process {} processing task of type {:?}", self.get_rank(), task.task_type);
        
        let graph = task.graph_partition.to_graph();
        
        match &task.task_type {
            GraphTaskType::DFS { start_node } => {
                let path = graph.dfs(*start_node);
                TaskResult {
                    path: Some(path),
                    distances: None,
                    has_negative_cycle: None,
                }
            },
            GraphTaskType::BFS { start_node } => {
                let path = graph.bfs(*start_node);
                TaskResult {
                    path: Some(path),
                    distances: None,
                    has_negative_cycle: None,
                }
            },
            GraphTaskType::Dijkstra { start_node } => {
                let (distances, path) = graph.dijkstra(*start_node);
                TaskResult {
                    path: Some(path),
                    distances: Some(distances),
                    has_negative_cycle: None,
                }
            },
            GraphTaskType::AStar { start_node, goal_node } => {
                let path = graph.astar(*start_node, *goal_node);
                TaskResult {
                    path: Some(path),
                    distances: None,
                    has_negative_cycle: None,
                }
            },
            GraphTaskType::BellmanFord { start_node } => {
                let (distances, has_negative_cycle) = graph.bellman_ford(*start_node);
                TaskResult {
                    path: None,
                    distances: Some(distances),
                    has_negative_cycle: Some(has_negative_cycle),
                }
            },
            GraphTaskType::Kruskal => {
                let mst = graph.kruskal();
                TaskResult {
                    path: Some(mst),
                    distances: None,
                    has_negative_cycle: None,
                }
            },
        }
    }
    
    // Merge results from different partitions
    fn merge_results(&self, master_result: &mut TaskResult, worker_result: &TaskResult) {
        println!("Master merging results from worker");
        
        // Merge paths
        if let (Some(master_path), Some(worker_path)) = (&mut master_result.path, &worker_result.path) {
            master_path.extend(worker_path.iter().cloned());
        }
        
        // Merge distances (taking minimum values)
        if let (Some(master_dist), Some(worker_dist)) = (&mut master_result.distances, &worker_result.distances) {
            for (i, &worker_value) in worker_dist.iter().enumerate() {
                if i < master_dist.len() {
                    master_dist[i] = master_dist[i].min(worker_value);
                } else {
                    master_dist.push(worker_value);
                }
            }
        }
        
        // Merge negative cycle detection (if any partition detects one, then there is one)
        if let (Some(master_cycle), Some(worker_cycle)) = (master_result.has_negative_cycle, worker_result.has_negative_cycle) {
            master_result.has_negative_cycle = Some(master_cycle || worker_cycle);
        }
    }
    
    // Clone task type for sending to workers
    fn clone_task_type(&self, task_type: &GraphTaskType) -> GraphTaskType {
        match task_type {
            GraphTaskType::DFS { start_node } => GraphTaskType::DFS { start_node: *start_node },
            GraphTaskType::BFS { start_node } => GraphTaskType::BFS { start_node: *start_node },
            GraphTaskType::Dijkstra { start_node } => GraphTaskType::Dijkstra { start_node: *start_node },
            GraphTaskType::AStar { start_node, goal_node } => 
                GraphTaskType::AStar { start_node: *start_node, goal_node: *goal_node },
            GraphTaskType::BellmanFord { start_node } => GraphTaskType::BellmanFord { start_node: *start_node },
            GraphTaskType::Kruskal => GraphTaskType::Kruskal,
        }
    }
}