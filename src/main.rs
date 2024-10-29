mod graph; // Ensure this module is correctly implemented
mod file_processor; // Ensure this module is correctly implemented

use axum::{extract::Multipart, routing::post, http::StatusCode, Router, Json, response::{IntoResponse, Response}, serve};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::path::Path;
use crate::file_processor::{process_file, FileFormat, ProcessError};
use crate::graph::Graph;

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

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("File processing error: {0}")]
    FileProcessing(#[from] ProcessError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid request: {0}")]
    BadRequest(String),
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::FileProcessing(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        };

        let body = Json(ProcessResponse {
            result: "error".to_string(),
            path: None,
            distances: None,
            error: Some(message),
        });

        (status, body).into_response()
    }
}

async fn process_graph_file(mut multipart: Multipart) -> Result<impl IntoResponse, AppError> {
    let mut file_data = Vec::new();
    let mut request: Option<ProcessRequest> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                file_data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?.to_vec();
            }
            "request" => {
                let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                request = Some(serde_json::from_slice(&data).map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            _ => {}
        }
    }

    let request = request.ok_or_else(|| AppError::BadRequest("Missing request data".to_string()))?;

    // Save file to temporary location
    let temp_path = format!("/tmp/{}", uuid::Uuid::new_v4());
    let mut temp_file = File::create(&temp_path).await?;
    temp_file.write_all(&file_data).await?;
    temp_file.flush().await?;

    // Process the file and run the algorithm
    let result = process_file_and_run_algorithm(&temp_path, request).await?;

    // Clean up temporary file
    tokio::fs::remove_file(&temp_path).await?;

    Ok(Json(result))
}

async fn process_file_and_run_algorithm(path: &str, request: ProcessRequest) -> Result<ProcessResponse, AppError> {
    // Ensure the graph is created from the processed file
    let mut graph = process_file(path, request.file_format)?; // Ensure this returns a Graph<usize, usize>

    let result = match request.algorithm.as_str() {
        "dfs" => {
            let start = request.start_node.unwrap_or(0); // Default to node 0 if not provided
            let path = graph.dfs(start); // Ensure this is defined in your Graph struct
            ProcessResponse {
                result: "DFS completed".to_string(),
                path: Some(path),
                distances: None,
                error: None,
            }
        },
        "bfs" => {
            let start = request.start_node.unwrap_or(0);
            let path = graph.bfs(start); // Ensure this is defined in your Graph struct
            ProcessResponse {
                result: "BFS completed".to_string(),
                path: Some(path),
                distances: None,
                error: None,
            }
        },
        "dijkstra" => {
            let start = request.start_node.unwrap_or(0);
            let (distances, path) = graph.dijkstra(start); // Ensure this is defined in your Graph struct
            ProcessResponse {
                result: "Dijkstra completed".to_string(),
                path: Some(path),
                distances: Some(distances),
                error: None,
            }
        },
        "astar" => {
            let start = request.start_node.unwrap_or(0);
            let end = request.end_node.unwrap_or(0);
            let path = graph.astar(start, end); // Ensure this is defined in your Graph struct
            ProcessResponse {
                result: "A* completed".to_string(),
                path: Some(path),
                distances: None,
                error: None,
            }
        },
        "bellman-ford" => {
            let start = request.start_node.unwrap_or(0);
            let (distances, has_negative_cycle) = graph.bellman_ford(start); // Ensure this is defined in your Graph struct
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
            let mst = graph.kruskal(); // Ensure this is defined in your Graph struct
            ProcessResponse {
                result: "Kruskal's MST completed".to_string(),
                path: Some(mst),
                distances: None,
                error: None,
            }
        },
        _ => return Err(AppError::BadRequest("Invalid algorithm specified".to_string())),
    };

    Ok(result)
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with a route
    let app = Router::new()
        .route("/process_file", post(process_graph_file))
        .layer(cors);

    // Run our application
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    axum_server::Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}