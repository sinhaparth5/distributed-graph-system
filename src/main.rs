use axum::{
    extract::Multipart,
    routing::post,
    http::StatusCode,
    Router,
    Json,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
// Import from the crate root instead of using crate::
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

    let temp_path = format!("/tmp/{}", uuid::Uuid::new_v4());
    let mut temp_file = File::create(&temp_path).await?;
    temp_file.write_all(&file_data).await?;
    temp_file.flush().await?;

    let result = process_file_and_run_algorithm(&temp_path, request).await?;

    tokio::fs::remove_file(&temp_path).await?;

    Ok(Json(result))
}

async fn process_file_and_run_algorithm(path: &str, request: ProcessRequest) -> Result<ProcessResponse, AppError> {
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
            let end = request.end_node.ok_or_else(|| AppError::BadRequest("End node required for A*".to_string()))?;
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
        _ => return Err(AppError::BadRequest("Invalid algorithm specified".to_string())),
    };

    Ok(result)
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/process_file", post(process_graph_file))
        .layer(cors);

    // Change from 127.0.0.1 to 0.0.0.0 to bind to all interfaces
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    
    axum::serve(listener, app).await.unwrap();
}