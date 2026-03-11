use mpi::point_to_point::{Source, Status};
use mpi::topology::Rank;
use mpi::traits::*;
use mpi::{self, request::WaitGuard};
use serde::{Deserialize, Serialize};
use bincode::{serialize, deserialize};
use mpi::environment::Threading;

use crate::graph::{Graph, Edge, Node, NodeFeatures};
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
    PageRank { damping: f64, iterations: u32 },
    SCC,
    TopologicalSort,
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
    pub components: Option<Vec<Vec<usize>>>,
    pub scores: Option<Vec<(usize, f64)>>,
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

// Safety: MPIProcessor is initialised with Threading::Multiple so OpenMPI
// allows concurrent use from different threads within the same process.
unsafe impl Send for MPIProcessor {}
unsafe impl Sync for MPIProcessor {}

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

    pub fn mode_name(&self) -> &'static str {
        match &self.mode {
            ExecutionMode::Distributed { .. } => "distributed",
            ExecutionMode::SingleProcess => "single-process",
        }
    }

    /// Called on worker processes (rank > 0). Blocks forever, handling one
    /// task per request that the master dispatches.
    pub fn run_worker_loop(&self) {
        println!("[MPI] Worker process {} ready, waiting for tasks...", self.get_rank());
        loop {
            self.execute_worker();
        }
    }

    // Build a full-graph partition. Every process receives the complete graph
    // so algorithms that traverse all nodes (BFS, DFS, Dijkstra, etc.) produce
    // correct results. The distribution is in the MPI communication: the master
    // serialises the graph, sends it over the network, the worker computes it,
    // and sends the result back.
    pub fn partition_graph(&self, graph: &Graph) -> Vec<GraphPartition> {
        let all_nodes: Vec<Node> = graph.get_nodes().values().cloned().collect();
        let all_edges: Vec<Edge> = graph.get_all_edges();
        let total_nodes = all_nodes.len();

        let mut all_node_features = HashMap::new();
        let mut feature_descriptions = HashMap::new();
        let mut ego_features = HashMap::new();

        for &node_id in graph.get_nodes().keys() {
            if let Some(features) = graph.get_node_features(node_id) {
                all_node_features.insert(node_id, features);
            }
        }
        for feature_id in 0..1000 {
            if let Some(desc) = graph.get_feature_description(feature_id) {
                feature_descriptions.insert(feature_id, desc.clone());
            }
        }
        if let Some(features) = graph.get_ego_features(0) {
            ego_features.insert(0, features.clone());
        }

        let full_partition = GraphPartition {
            nodes: all_nodes,
            edges: all_edges,
            node_range: (0, total_nodes),
            node_features: all_node_features,
            feature_descriptions,
            ego_features,
        };

        // One full-graph copy per process
        let num_processes = self.get_size() as usize;
        (0..num_processes).map(|_| full_partition.clone()).collect()
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
                        components: None,
                        scores: None,
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
                    components: None,
                    scores: None,
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
    
    // Worker process: receive task, compute, send back result.
    // Panics are caught so the worker always sends a reply — without this,
    // a worker crash leaves the master blocked in receive_vec forever.
    fn execute_worker(&self) -> TaskResult {
        if let ExecutionMode::Distributed { world, .. } = &self.mode {
            let (task_data, _): (Vec<u8>, Status) = world.process_at_rank(0)
                .receive_vec::<u8>();

            let task: GraphTask = match deserialize(&task_data) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("[MPI] Worker {} failed to deserialise task: {}", self.get_rank(), e);
                    let empty = TaskResult { path: Some(vec![]), distances: None, has_negative_cycle: None, components: None, scores: None };
                    let serialized = serialize(&empty).unwrap_or_default();
                    world.process_at_rank(0).send(&serialized[..]);
                    return empty;
                }
            };

            println!("[MPI] Worker {} received task: {} nodes, {} edges",
                     self.get_rank(), task.graph_partition.nodes.len(), task.graph_partition.edges.len());

            // Catch panics so we always send a result back to unblock master
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.process_task(&task)
            })).unwrap_or_else(|e| {
                eprintln!("[MPI] Worker {} task panicked: {:?}", self.get_rank(), e);
                TaskResult { path: Some(vec![]), distances: None, has_negative_cycle: None, components: None, scores: None }
            });

            let serialized = serialize(&result).expect("Failed to serialise result");
            world.process_at_rank(0).send(&serialized[..]);
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
                    components: None,
                    scores: None,
                }
            },
            GraphTaskType::BFS { start_node } => {
                let path = graph.bfs(*start_node);
                TaskResult {
                    path: Some(path),
                    distances: None,
                    has_negative_cycle: None,
                    components: None,
                    scores: None,
                }
            },
            GraphTaskType::Dijkstra { start_node } => {
                let (distances, path) = graph.dijkstra(*start_node);
                TaskResult {
                    path: Some(path),
                    distances: Some(distances),
                    has_negative_cycle: None,
                    components: None,
                    scores: None,
                }
            },
            GraphTaskType::AStar { start_node, goal_node } => {
                let path = graph.astar(*start_node, *goal_node);
                TaskResult {
                    path: Some(path),
                    distances: None,
                    has_negative_cycle: None,
                    components: None,
                    scores: None,
                }
            },
            GraphTaskType::BellmanFord { start_node } => {
                let (distances, has_negative_cycle) = graph.bellman_ford(*start_node);
                TaskResult {
                    path: None,
                    distances: Some(distances),
                    has_negative_cycle: Some(has_negative_cycle),
                    components: None,
                    scores: None,
                }
            },
            GraphTaskType::Kruskal => {
                let mst = graph.kruskal();
                TaskResult {
                    path: Some(mst),
                    distances: None,
                    has_negative_cycle: None,
                    components: None,
                    scores: None,
                }
            },
            GraphTaskType::PageRank { damping, iterations } => {
                let scores = graph.pagerank(*damping, *iterations);
                TaskResult { path: None, distances: None, has_negative_cycle: None, components: None, scores: Some(scores) }
            },
            GraphTaskType::SCC => {
                let comps = graph.scc();
                // Also flatten into path so frontend can highlight nodes
                let path: Vec<usize> = comps.iter().flat_map(|c| c.iter().copied()).collect();
                TaskResult { path: Some(path), distances: None, has_negative_cycle: None, components: Some(comps), scores: None }
            },
            GraphTaskType::TopologicalSort => {
                let order = graph.topological_sort().unwrap_or_default();
                TaskResult { path: Some(order), distances: None, has_negative_cycle: None, components: None, scores: None }
            },
        }
    }
    
    // Merge results. Both processes ran on the full graph so results are
    // equivalent — prefer the worker's result to demonstrate that computation
    // was actually performed on the remote worker node.
    fn merge_results(&self, master_result: &mut TaskResult, worker_result: &TaskResult) {
        println!("[MPI] Master merging result from worker");

        if let Some(worker_path) = &worker_result.path {
            if !worker_path.is_empty() {
                master_result.path = Some(worker_path.clone());
            }
        }
        if let Some(worker_dist) = &worker_result.distances {
            if !worker_dist.is_empty() {
                master_result.distances = Some(worker_dist.clone());
            }
        }
        if let Some(worker_cycle) = worker_result.has_negative_cycle {
            master_result.has_negative_cycle = Some(worker_cycle);
        }
        if let Some(worker_comps) = &worker_result.components {
            if !worker_comps.is_empty() {
                master_result.components = Some(worker_comps.clone());
            }
        }
        if let Some(worker_scores) = &worker_result.scores {
            if !worker_scores.is_empty() {
                master_result.scores = Some(worker_scores.clone());
            }
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
            GraphTaskType::PageRank { damping, iterations } => GraphTaskType::PageRank { damping: *damping, iterations: *iterations },
            GraphTaskType::SCC => GraphTaskType::SCC,
            GraphTaskType::TopologicalSort => GraphTaskType::TopologicalSort,
        }
    }
}