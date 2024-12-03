#[macro_use] extern crate rocket;

use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{post, routes};
use serde::{Deserialize, Serialize};
use rocket::form::Form;
use tempfile::NamedTempFile;
use dristributed_graph_system::file_processor::{process_file, FileFormat, ProcessError};

#[derive(Debug, Serialize, Deserialize)]
struct ProcessRequest {
    algorithm: String,
    file_format: FileFormat,
    start_node: Option<usize>,
    end_node: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ProcessResponse {
    result: String,
    path: Option<Vec<usize>>,
    distances: Option<Vec<f64>>,
    error: Option<String>,
}

#[derive(Debug, FromForm)]
struct UploadForm<'f> {
    file: TempFile<'f>,
    request: String,
}

// Explicitly specify the lifetime parameter
#[post("/process_file", data = "<form>")]
async fn process_graph_file<'f>(mut form: Form<UploadForm<'f>>) -> Result<Json<ProcessResponse>, Status> {
    println!("Received request data: {}", form.request);

    let request: ProcessRequest = match serde_json::from_str(&form.request) {
        Ok(req) => req,
        Err(e) => {
            println!("JSON parsing error: {}", e);
            return Ok(Json(ProcessResponse {
                result: "error".to_string(),
                path: None,
                distances: None,
                error: Some(format!("Invalid request format: {}", e)),
            }));
        }
    };

    let temp_file = match NamedTempFile::new() {
        Ok(file) => file,
        Err(e) => {
            println!("Temp file creation error: {}", e);
            return Ok(Json(ProcessResponse {
                result: "error".to_string(),
                path: None,
                distances: None,
                error: Some("Failed to create temporary file".to_string()),
            }));
        }
    };

    if let Err(e) = form.file.persist_to(temp_file.path()).await {
        println!("File persistence error: {}", e);
        return Ok(Json(ProcessResponse {
            result: "error".to_string(),
            path: None,
            distances: None,
            error: Some("Failed to save uploaded file".to_string()),
        }));
    }

    match process_file_and_run_algorithm(temp_file.path().to_str().unwrap(), request) {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            println!("Processing error: {:?}", e);
            Ok(Json(ProcessResponse {
                result: "error".to_string(),
                path: None,
                distances: None,
                error: Some(format!("Processing error: {:?}", e)),
            }))
        }
    }
}

fn process_file_and_run_algorithm(path: &str, request: ProcessRequest) -> Result<ProcessResponse, ProcessError> {
    let graph = process_file(path, request.file_format)?;

    let result = match request.algorithm.as_str() {
        "dfs" => {
            let start = request.start_node.unwrap_or(0);
            let path = graph.dfs(start);
            ProcessResponse {
                result: "DFS completed".to_string(),
                path: Some(path),
                distances: None,
                error: None,
            }
        },
        "bfs" => {
            let start = request.start_node.unwrap_or(0);
            let path = graph.bfs(start);
            ProcessResponse {
                result: "BFS completed".to_string(),
                path: Some(path),
                distances: None,
                error: None,
            }
        },
        "dijkstra" => {
            let start = request.start_node.unwrap_or(0);
            let (distances, path) = graph.dijkstra(start);
            ProcessResponse {
                result: "Dijkstra completed".to_string(),
                path: Some(path),
                distances: Some(distances),
                error: None,
            }
        },
        "astar" => {
            let start = request.start_node.unwrap_or(0);
            let end = request.end_node.ok_or(ProcessError::ParsingError("End node required for A*".to_string()))?;
            let path = graph.astar(start, end);
            ProcessResponse {
                result: "A* completed".to_string(),
                path: Some(path),
                distances: None,
                error: None,
            }
        },
        "bellman-ford" => {
            let start = request.start_node.unwrap_or(0);
            let (distances, has_negative_cycle) = graph.bellman_ford(start);
            ProcessResponse {
                result: if has_negative_cycle {
                    "Negative cycle detected".to_string()
                } else {
                    "Bellman-Ford completed".to_string()
                },
                path: None,
                distances: Some(distances),
                error: None,
            }
        },
        "kruskal" => {
            let mst = graph.kruskal();
            ProcessResponse {
                result: "Kruskal's MST completed".to_string(),
                path: Some(mst),
                distances: None,
                error: None,
            }
        },
        _ => return Err(ProcessError::InvalidFormat),
    };

    Ok(result)
}

#[launch]
fn rocket() -> _ {
    let figment = rocket::Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge(("port", 8000));

    let cors = rocket::fairing::AdHoc::on_response("CORS", |_, res| {
        Box::pin(async move {
            res.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", "*"));
            res.set_header(rocket::http::Header::new("Access-Control-Allow-Methods", "POST, GET, OPTIONS"));
            res.set_header(rocket::http::Header::new("Access-Control-Allow-Headers", "*"));
        })
    });

    rocket::custom(figment)
        .attach(cors)
        .mount("/", routes![process_graph_file])
}