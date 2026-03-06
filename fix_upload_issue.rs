// src/bin/fixed_upload_issue.rs
#[macro_use] extern crate rocket;

use rocket::fs::TempFile;
use rocket::form::Form;
use rocket::serde::json::Json;
use rocket::tokio::io::AsyncReadExt;  
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(FromForm)]
struct UploadForm<'f> {
    file: TempFile<'f>,
    request: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessRequest {
    algorithm: String,
    #[serde(rename = "file_format")]
    file_format: String,
    start_node: Option<usize>,
    end_node: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ProcessResponse {
    result: String,
    path: Option<Vec<usize>>,
    distances: Option<Vec<f64>>,
    error: Option<String>,
    debug_info: DebugInfo,
}

#[derive(Debug, Serialize)]
struct DebugInfo {
    request_content: String,
    file_name: Option<String>,
    file_content_type: Option<String>,
    tmp_dir_exists: bool,
    tmp_dir_writable: bool,
    file_save_location: String,
    persist_error: Option<String>,  // Added this field for the error message
}

#[post("/process_file", data = "<form>")]
async fn process_file(mut form: Form<UploadForm<'_>>) -> Json<ProcessResponse> {
    // Debug info
    let mut debug_info = DebugInfo {
        request_content: form.request.clone(),
        file_name: form.file.name().map(|s| s.to_string()),
        file_content_type: form.file.content_type().map(|ct| ct.to_string()),
        tmp_dir_exists: false,
        tmp_dir_writable: false,
        file_save_location: String::new(),
        persist_error: None,  // Initialize the new field
    };

    // Create and check /tmp directory
    let temp_dir = PathBuf::from("/tmp");
    debug_info.tmp_dir_exists = temp_dir.exists();
    
    // Test write to temp dir
    let test_file_path = temp_dir.join("test_write.txt");
    match fs::write(&test_file_path, "test") {
        Ok(_) => {
            debug_info.tmp_dir_writable = true;
            let _ = fs::remove_file(&test_file_path);
        }
        Err(_) => debug_info.tmp_dir_writable = false,
    }

    // Ensure the directory exists
    if !temp_dir.exists() {
        if let Err(_) = fs::create_dir_all(&temp_dir) {
            return Json(ProcessResponse {
                result: "error".to_string(),
                path: None,
                distances: None,
                error: Some("Failed to create temp directory".to_string()),
                debug_info,
            });
        }
    }

    // Try to save the file directly with persist_to first
    let file_path = temp_dir.join("uploaded_file.txt");
    debug_info.file_save_location = file_path.to_string_lossy().to_string();

    match form.file.persist_to(&file_path).await {
        Ok(_) => {
            // Verify file was saved
            match fs::read_to_string(&file_path) {
                Ok(_) => {
                    // Success!
                    return Json(ProcessResponse {
                        result: "success".to_string(),
                        path: Some(vec![0, 1, 2]), // Dummy path
                        distances: None,
                        error: None,
                        debug_info,
                    });
                },
                Err(e) => {
                    return Json(ProcessResponse {
                        result: "error".to_string(),
                        path: None,
                        distances: None,
                        error: Some(format!("File saved but could not be read: {}", e)),
                        debug_info,
                    });
                }
            }
        },
        Err(e) => {
            // Use the new field for the error message
            debug_info.persist_error = Some(format!("persist_to failed: {}", e));
            // Fall through to alternative method
        }
    }

    // Alternative: Manual file save approach
    let mut file_content = Vec::new();
    let read_result = match form.file.open().await {
        Ok(mut opened_file) => {
            match opened_file.read_to_end(&mut file_content).await {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to read uploaded file: {}", e))
            }
        },
        Err(e) => Err(format!("Failed to open uploaded file: {}", e))
    };

    if let Err(e) = read_result {
        return Json(ProcessResponse {
            result: "error".to_string(),
            path: None,
            distances: None,
            error: Some(e),
            debug_info,
        });
    }

    // Write the file to disk
    match fs::write(&file_path, &file_content) {
        Ok(_) => (),
        Err(e) => {
            return Json(ProcessResponse {
                result: "error".to_string(),
                path: None,
                distances: None,
                error: Some(format!("Failed to write file to disk: {}", e)),
                debug_info,
            });
        }
    }

    // Verify file was saved
    match fs::read_to_string(&file_path) {
        Ok(_) => {
            // Success!
            Json(ProcessResponse {
                result: "success".to_string(),
                path: Some(vec![0, 1, 2]), // Dummy path
                distances: None,
                error: None,
                debug_info,
            })
        },
        Err(e) => {
            Json(ProcessResponse {
                result: "error".to_string(),
                path: None,
                distances: None,
                error: Some(format!("File saved but could not be read: {}", e)),
                debug_info,
            })
        }
    }
}

#[get("/health")]
fn health() -> &'static str {
    "OK"
}

#[launch]
fn rocket() -> _ {
    println!("Starting upload fix server on port 8000");
    
    rocket::build()
        .configure(rocket::Config::figment()
            .merge(("address", "0.0.0.0"))
            .merge(("port", 8000)))
        .mount("/", routes![health, process_file])
}