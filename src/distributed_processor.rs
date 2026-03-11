use crate::file_processor::{process_file, FileFormat};
use crate::graph::Graph;
use crate::mpi_processor::{MPIProcessor, GraphTaskType, TaskResult};

pub struct AlgorithmResult {
    pub task_result: TaskResult,
    pub mpi_processes: usize,
    pub mpi_mode: String,
}

pub fn run_distributed_algorithm(
    mpi: &MPIProcessor,
    file_path: &str,
    algorithm: &str,
    file_format: FileFormat,
    start_node: Option<usize>,
    end_node: Option<usize>,
) -> Result<AlgorithmResult, String> {
    let graph = if mpi.is_master() {
        match process_file(file_path, file_format) {
            Ok(g) => g,
            Err(e) => return Err(format!("File processing error: {:?}", e)),
        }
    } else {
        Graph::new()
    };

    let task_type = match algorithm {
        "dfs"          => GraphTaskType::DFS { start_node: start_node.unwrap_or(0) },
        "bfs"          => GraphTaskType::BFS { start_node: start_node.unwrap_or(0) },
        "dijkstra"     => GraphTaskType::Dijkstra { start_node: start_node.unwrap_or(0) },
        "astar"        => {
            let goal = end_node.ok_or_else(|| "End node required for A* algorithm".to_string())?;
            GraphTaskType::AStar { start_node: start_node.unwrap_or(0), goal_node: goal }
        },
        "bellman-ford" => GraphTaskType::BellmanFord { start_node: start_node.unwrap_or(0) },
        "kruskal"      => GraphTaskType::Kruskal,
        "pagerank"         => GraphTaskType::PageRank { damping: 0.85, iterations: 30 },
        "scc"              => GraphTaskType::SCC,
        "topological-sort" => GraphTaskType::TopologicalSort,
        _              => return Err(format!("Unsupported algorithm: {}", algorithm)),
    };

    let task_result = mpi.execute_distributed_algorithm(&graph, task_type);

    Ok(AlgorithmResult {
        task_result,
        mpi_processes: mpi.get_size() as usize,
        mpi_mode: mpi.mode_name().to_string(),
    })
}
