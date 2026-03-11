#[macro_use] extern crate rocket;

use rocket::fs::TempFile;
use rocket::serde::json::Json;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response, State};
use rocket::form::Form;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use distributed_graph_system::file_processor::FileFormat;
use distributed_graph_system::file_processor::process_file;
use distributed_graph_system::distributed_processor::run_distributed_algorithm;
use distributed_graph_system::mpi_processor::MPIProcessor;

// ── CORS ─────────────────────────────────────────────────────────────────────

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info { name: "CORS", kind: Kind::Response }
    }
    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        res.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, OPTIONS"));
        res.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        res.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct ProcessRequest {
    algorithm: String,
    #[serde(rename = "file_format")]
    file_format: FileFormat,
    start_node: Option<usize>,
    end_node: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ProcessResponse {
    result: String,
    path: Option<Vec<usize>>,
    distances: Option<Vec<f64>>,
    has_negative_cycle: Option<bool>,
    components: Option<Vec<Vec<usize>>>,
    scores: Option<Vec<(usize, f64)>>,
    error: Option<String>,
    mpi_processes: usize,
    mpi_mode: String,
}

#[derive(Debug, Serialize)]
struct MpiStatusResponse {
    mpi_processes: usize,
    mpi_mode: String,
    master_rank: i32,
    note: String,
}

#[derive(Debug, FromForm)]
struct UploadForm<'f> {
    file: TempFile<'f>,
    request: String,
}

// ── Routes ───────────────────────────────────────────────────────────────────

#[options("/<_..>")]
fn options() -> &'static str { "" }

#[get("/")]
fn index() -> &'static str { "Distributed Graph Processing API" }

#[get("/health")]
fn health_check() -> &'static str { "OK" }

#[get("/mpi_status")]
fn mpi_status(mpi: &State<Arc<MPIProcessor>>) -> Json<MpiStatusResponse> {
    let processes = mpi.get_size() as usize;
    let mode = mpi.mode_name().to_string();
    let note = if processes > 1 {
        format!("{} MPI worker(s) connected and ready to receive graph partitions.", processes - 1)
    } else {
        "Running with 1 MPI process (no workers). \
         Start with mpirun to distribute across worker containers.".to_string()
    };
    Json(MpiStatusResponse {
        mpi_processes: processes,
        mpi_mode: mode,
        master_rank: mpi.get_rank(),
        note,
    })
}

#[post("/process_file", data = "<form>")]
async fn process_graph_file<'f>(
    mpi: &State<Arc<MPIProcessor>>,
    mut form: Form<UploadForm<'f>>,
) -> Json<ProcessResponse> {
    let request: ProcessRequest = match serde_json::from_str(&form.request) {
        Ok(r) => r,
        Err(e) => return Json(ProcessResponse {
            result: "error".to_string(),
            path: None, distances: None,
            has_negative_cycle: None, components: None, scores: None,
            error: Some(format!("Invalid request JSON: {}", e)),
            mpi_processes: mpi.get_size() as usize,
            mpi_mode: mpi.mode_name().to_string(),
        }),
    };

    let temp_file_path = PathBuf::from("/tmp")
        .join(format!("graph_{}.txt", uuid::Uuid::new_v4()));

    if let Err(e) = form.file.persist_to(&temp_file_path).await {
        return Json(ProcessResponse {
            result: "error".to_string(),
            path: None, distances: None,
            has_negative_cycle: None, components: None, scores: None,
            error: Some(format!("Failed to save file: {}", e)),
            mpi_processes: mpi.get_size() as usize,
            mpi_mode: mpi.mode_name().to_string(),
        });
    }

    // MPI calls are blocking — run off the async thread so the Rocket runtime
    // stays responsive while the master/worker exchange is happening.
    let mpi_arc = Arc::clone(mpi);
    let path_str   = temp_file_path.to_str().unwrap().to_string();
    let algorithm  = request.algorithm.clone();
    let file_format = request.file_format.clone();
    let start_node = request.start_node;
    let end_node   = request.end_node;

    let result = tokio::task::spawn_blocking(move || {
        run_distributed_algorithm(&mpi_arc, &path_str, &algorithm,
                                  file_format, start_node, end_node)
    }).await;

    let _ = std::fs::remove_file(&temp_file_path);

    match result {
        Ok(Ok(algo)) => {
            let message = match request.algorithm.as_str() {
                "dfs"          => "DFS completed",
                "bfs"          => "BFS completed",
                "dijkstra"     => "Dijkstra completed",
                "astar"        => "A* completed",
                "bellman-ford" => {
                    if algo.task_result.has_negative_cycle.unwrap_or(false) {
                        "Negative cycle detected"
                    } else {
                        "Bellman-Ford completed"
                    }
                },
                "kruskal" => "Kruskal's MST completed",
                _         => "Algorithm completed",
            };
            println!("[MPI] {} — {} process(es), {} mode",
                     message, algo.mpi_processes, algo.mpi_mode);
            Json(ProcessResponse {
                result: message.to_string(),
                path: algo.task_result.path,
                distances: algo.task_result.distances,
                has_negative_cycle: algo.task_result.has_negative_cycle,
                components: algo.task_result.components,
                scores: algo.task_result.scores,
                error: None,
                mpi_processes: algo.mpi_processes,
                mpi_mode: algo.mpi_mode,
            })
        },
        Ok(Err(e)) => Json(ProcessResponse {
            result: "error".to_string(),
            path: None, distances: None,
            has_negative_cycle: None, components: None, scores: None,
            error: Some(e),
            mpi_processes: mpi.get_size() as usize,
            mpi_mode: mpi.mode_name().to_string(),
        }),
        Err(e) => Json(ProcessResponse {
            result: "error".to_string(),
            path: None, distances: None,
            has_negative_cycle: None, components: None, scores: None,
            error: Some(format!("Task panicked: {}", e)),
            mpi_processes: mpi.get_size() as usize,
            mpi_mode: mpi.mode_name().to_string(),
        }),
    }
}

#[derive(Debug, Serialize)]
struct HubInfo {
    id: usize,
    degree: usize,
}

#[derive(Debug, Serialize)]
struct GraphMetricsResponse {
    node_count: usize,
    edge_count: usize,
    density: f64,
    connected_components: usize,
    is_dag: bool,
    avg_degree: f64,
    top_hubs: Vec<HubInfo>,
    error: Option<String>,
}

#[derive(Debug, FromForm)]
struct MetricsForm<'f> {
    file: TempFile<'f>,
    file_format: String,
}

#[post("/graph_metrics", data = "<form>")]
async fn graph_metrics_route<'f>(
    mut form: Form<MetricsForm<'f>>,
) -> Json<GraphMetricsResponse> {
    let fmt_str = form.file_format.trim().to_string();
    let file_format = match fmt_str.as_str() {
        "edgeList"      => distributed_graph_system::file_processor::FileFormat::EdgeList,
        "adjacencyList" => distributed_graph_system::file_processor::FileFormat::AdjacencyList,
        other => return Json(GraphMetricsResponse {
            node_count: 0, edge_count: 0, density: 0.0,
            connected_components: 0, is_dag: false, avg_degree: 0.0,
            top_hubs: vec![],
            error: Some(format!("Unknown file_format: {}", other)),
        }),
    };

    let temp_path = PathBuf::from("/tmp")
        .join(format!("metrics_{}.txt", uuid::Uuid::new_v4()));
    if let Err(e) = form.file.persist_to(&temp_path).await {
        return Json(GraphMetricsResponse {
            node_count: 0, edge_count: 0, density: 0.0,
            connected_components: 0, is_dag: false, avg_degree: 0.0,
            top_hubs: vec![],
            error: Some(format!("Failed to save file: {}", e)),
        });
    }

    let path_str = temp_path.to_str().unwrap().to_string();
    let result = tokio::task::spawn_blocking(move || {
        process_file(&path_str, file_format)
    }).await;
    let _ = std::fs::remove_file(&temp_path);

    match result {
        Ok(Ok(graph)) => {
            let n = graph.node_count();
            let e = graph.edge_count();
            let density = if n > 1 { e as f64 / (n * (n - 1)) as f64 } else { 0.0 };
            let cc = graph.connected_components();
            let is_dag = graph.topological_sort().is_some();
            let avg = if n > 0 { e as f64 / n as f64 } else { 0.0 };
            let hubs = graph.top_hubs(5).into_iter()
                .map(|(id, deg)| HubInfo { id, degree: deg })
                .collect();
            Json(GraphMetricsResponse {
                node_count: n, edge_count: e, density,
                connected_components: cc, is_dag, avg_degree: avg,
                top_hubs: hubs, error: None,
            })
        },
        Ok(Err(e)) => Json(GraphMetricsResponse {
            node_count: 0, edge_count: 0, density: 0.0,
            connected_components: 0, is_dag: false, avg_degree: 0.0,
            top_hubs: vec![], error: Some(format!("{}", e)),
        }),
        Err(e) => Json(GraphMetricsResponse {
            node_count: 0, edge_count: 0, density: 0.0,
            connected_components: 0, is_dag: false, avg_degree: 0.0,
            top_hubs: vec![], error: Some(format!("{}", e)),
        }),
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────
//
// Rocket's #[launch] is NOT used here because we need to inspect MPI rank
// before deciding whether to start the web server or a worker loop.

fn main() {
    // Initialise MPI once for the lifetime of this process.
    let mpi = Arc::new(MPIProcessor::new());

    println!("[MPI] Process {} of {} started — mode: {}",
             mpi.get_rank(), mpi.get_size(), mpi.mode_name());

    if mpi.is_master() {
        // ── Rank 0: run the Rocket web server ──────────────────────────────
        println!("[MPI] This is the master node. Starting web server on :8000");

        let figment = rocket::Config::figment()
            .merge(("address", "0.0.0.0"))
            .merge(("port", 8000))
            .merge(("log_level", "normal"));

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                rocket::custom(figment)
                    .attach(CORS)
                    .manage(mpi)
                    .mount("/", routes![
                        index, health_check, mpi_status, process_graph_file,
                        graph_metrics_route, options
                    ])
                    .launch()
                    .await
                    .expect("Rocket server failed");
            });
    } else {
        // ── Rank > 0: blocking worker loop ─────────────────────────────────
        // Each iteration receives one graph partition, processes it, and sends
        // the result back to the master. Loop stays alive between requests.
        mpi.run_worker_loop();
    }
}
