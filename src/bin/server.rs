#[macro_use] extern crate rocket;

use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};
use rocket::form::Form;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Import necessary items
use distributed_graph_system::file_processor::{FileFormat, ProcessError};
use distributed_graph_system::distributed_processor::run_distributed_algorithm;

// CORS Fairing
pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

// Request and response structures
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

// Catch OPTIONS requests for CORS preflight
#[options("/<_..>")]
fn options() -> &'static str {
    ""
}

// Basic health check endpoint
#[get("/health")]
fn health_check() -> &'static str {
    "OK"
}

// Root endpoint
#[get("/")]
fn index() -> &'static str {
    "Distributed Graph Processing API"
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

    // Use /tmp directory which is guaranteed to be writable
    let temp_dir = PathBuf::from("/tmp");
    
    // Create temporary file path with UUID to avoid conflicts
    let filename = format!("graph_upload_{}.txt", uuid::Uuid::new_v4());
    let temp_file_path = temp_dir.join(&filename);
    
    println!("Will save file to: {:?}", temp_file_path);

    // Persist the file
    if let Err(e) = form.file.persist_to(&temp_file_path).await {
        println!("File persistence error: {}", e);
        return Ok(Json(ProcessResponse {
            result: "error".to_string(),
            path: None,
            distances: None,
            error: Some(format!("Failed to save uploaded file: {}", e)),
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
    println!("Starting Rocket server on 0.0.0.0:8000...");
    
    // Configure Rocket
    let figment = rocket::Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge(("port", 8000))
        .merge(("log_level", "normal"));

    rocket::custom(figment)
        .attach(CORS)
        .mount("/", routes![index, health_check, process_graph_file, options])
}
