use crate::file_processor::{process_file, FileFormat};
use crate::graph::Graph;
use crate::mpi_processor::{MPIProcessor, GraphTaskType, TaskResult};

pub struct DistributedGraphProcessor {
    mpi: MPIProcessor,
}

impl DistributedGraphProcessor {
    pub fn new() -> Self {
        DistributedGraphProcessor {
            mpi: MPIProcessor::new(),
        }
    }

    pub fn process(&self, file_path: &str, file_format: FileFormat, algorithm: &str,
                   start_node: Option<usize>, end_node: Option<usize>) -> Result<TaskResult, String> {
        let graph = if self.mpi.is_master() {
            match process_file(file_path, file_format) {
                Ok(g) => g,
                Err(e) => return Err(format!("File processing error: {:?}", e)),
            }
        } else {
            Graph::new()
        };

        self.execute_algorithm(&graph, algorithm, start_node, end_node)
    }

    // Helper method to execute an algorithm on a graph
    fn execute_algorithm(&self, graph: &Graph, algorithm: &str,
                        start_node: Option<usize>, end_node: Option<usize>) -> Result<TaskResult, String> {
        // Determine the task type
        let task_type = match algorithm {
            "dfs" => GraphTaskType::DFS { 
                start_node: start_node.unwrap_or(0) 
            },
            "bfs" => GraphTaskType::BFS { 
                start_node: start_node.unwrap_or(0) 
            },
            "dijkstra" => GraphTaskType::Dijkstra { 
                start_node: start_node.unwrap_or(0) 
            },
            "astar" => {
                let goal = match end_node {
                    Some(node) => node,
                    None => return Err("End node required for A* algorithm".to_string()),
                };
                GraphTaskType::AStar { 
                    start_node: start_node.unwrap_or(0),
                    goal_node: goal,
                }
            },
            "bellman-ford" => GraphTaskType::BellmanFord { 
                start_node: start_node.unwrap_or(0) 
            },
            "kruskal" => GraphTaskType::Kruskal,
            _ => return Err(format!("Unsupported algorithm: {}", algorithm)),
        };
        
        // Execute the distributed algorithm
        let result = self.mpi.execute_distributed_algorithm(graph, task_type);
        
        Ok(result)
    }
}

pub fn run_distributed_algorithm(
    file_path: &str,
    algorithm: &str,
    file_format: FileFormat,
    start_node: Option<usize>,
    end_node: Option<usize>,
) -> Result<TaskResult, String> {
    let processor = DistributedGraphProcessor::new();
    processor.process(file_path, file_format, algorithm, start_node, end_node)
}