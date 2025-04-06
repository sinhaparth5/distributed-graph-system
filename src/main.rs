#[macro_use] extern crate rocket;

use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{post, routes};
use serde::{Deserialize, Serialize};
use rocket::form::Form;
use distributed_graph_system::file_processor::{FileFormat, ProcessError};
use distributed_graph_system::distributed_processor::run_distributed_algorithm;
use std::env;
use std::path::PathBuf;

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
    error: Option<String>,
}

#[derive(Debug, FromForm)]
struct UploadForm<'f> {
    file: TempFile<'f>,
    request: String,
}

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

    // Create a temporary directory in the current directory
    let mut temp_dir = env::current_dir()
        .map_err(|e| {
            println!("Failed to get current directory: {}", e);
            Status::InternalServerError
        })?;
    temp_dir.push("temp");
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        println!("Failed to create temp directory: {}", e);
        Status::InternalServerError
    })?;

    // Create temporary file path
    let temp_file_path = temp_dir.join(format!("upload_{}.txt", uuid::Uuid::new_v4()));
    println!("Temp file path: {:?}", temp_file_path);

    // Persist the file
    if let Err(e) = form.file.persist_to(&temp_file_path).await {
        println!("File persistence error: {}", e);
        return Ok(Json(ProcessResponse {
            result: "error".to_string(),
            path: None,
            distances: None,
            error: Some("Failed to save uploaded file".to_string()),
        }));
    }

    // Process the file using the distributed system
    let result = match run_distributed_algorithm(
        temp_file_path.to_str().unwrap(),
        &request.algorithm,
        request.file_format,
        request.start_node,
        request.end_node
    ) {
        Ok(task_result) => {
            let message = match request.algorithm.as_str() {
                "dfs" => "DFS completed".to_string(),
                "bfs" => "BFS completed".to_string(),
                "dijkstra" => "Dijkstra completed".to_string(),
                "astar" => "A* completed".to_string(),
                "bellman-ford" => {
                    if task_result.has_negative_cycle.unwrap_or(false) {
                        "Negative cycle detected".to_string()
                    } else {
                        "Bellman-Ford completed".to_string()
                    }
                },
                "kruskal" => "Kruskal's MST completed".to_string(),
                _ => "Algorithm completed".to_string(),
            };

            ProcessResponse {
                result: message,
                path: task_result.path,
                distances: task_result.distances,
                error: None,
            }
        },
        Err(e) => {
            println!("Processing error: {}", e);
            ProcessResponse {
                result: "error".to_string(),
                path: None,
                distances: None,
                error: Some(format!("Processing error: {}", e)),
            }
        }
    };

    // Clean up
    if let Err(e) = std::fs::remove_file(&temp_file_path) {
        println!("Warning: Failed to remove temporary file: {}", e);
    }

    Ok(Json(result))
}

#[launch]
fn rocket() -> _ {
    // Create temp directory at startup
    let temp_dir = env::current_dir()
        .map(|mut path| {
            path.push("temp");
            std::fs::create_dir_all(&path).unwrap_or_else(|e| {
                println!("Warning: Failed to create temp directory: {}", e);
            });
            path
        })
        .unwrap_or_else(|e| {
            println!("Warning: Failed to get current directory: {}", e);
            PathBuf::from("temp")
        });

    println!("Using temp directory: {:?}", temp_dir);

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