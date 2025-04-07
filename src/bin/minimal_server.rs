#[macro_use] extern crate rocket;

use rocket::fs::TempFile;
use rocket::form::Form;
use rocket::serde::json::Json;
use serde::Serialize;
use std::path::PathBuf;

#[derive(FromForm)]
struct Upload<'r> {
    file: TempFile<'r>,
}

#[derive(Serialize)]
struct UploadResponse {
    success: bool,
    message: String,
    path: String,
    exists: bool,
    writable: bool,
}

#[post("/debug_upload", data = "<form>")]
async fn debug_upload(mut form: Form<Upload<'_>>) -> Json<UploadResponse> {
    println!("Debug upload request received");
    
    // Create fixed path without using env::current_dir()
    let temp_dir = PathBuf::from("/app/temp");
    
    // Check if directory exists
    let exists = temp_dir.exists();
    println!("Temp directory exists: {}", exists);
    
    // Check if directory is writable
    let writable = if exists {
        match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(temp_dir.join("write_test.txt"))
        {
            Ok(_) => {
                println!("Temp directory is writable");
                true
            },
            Err(e) => {
                println!("Temp directory is not writable: {}", e);
                false
            }
        }
    } else {
        false
    };
    
    // Create directory if it doesn't exist
    if !exists {
        println!("Creating temp directory");
        if let Err(e) = std::fs::create_dir_all(&temp_dir) {
            println!("Failed to create temp directory: {}", e);
            return Json(UploadResponse {
                success: false,
                message: format!("Failed to create temp directory: {}", e),
                path: temp_dir.to_string_lossy().to_string(),
                exists: false,
                writable: false,
            });
        }
    }
    
    // Use a fixed filename for simplicity
    let filename = "uploaded_file.txt";
    let file_path = temp_dir.join(filename);
    println!("Will save file to: {:?}", file_path);
    
    // Try to persist the file
    match form.file.persist_to(&file_path).await {
        Ok(_) => {
            println!("File persisted successfully to {:?}", file_path);
            
            // Read back the file
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    let preview = if content.len() > 100 {
                        format!("{}...", &content[..100])
                    } else {
                        content.clone()
                    };
                    
                    println!("File content: {}", preview);
                    
                    Json(UploadResponse {
                        success: true,
                        message: format!("File uploaded and read successfully. Content: {}", preview),
                        path: file_path.to_string_lossy().to_string(),
                        exists,
                        writable,
                    })
                },
                Err(e) => {
                    println!("Failed to read file: {}", e);
                    Json(UploadResponse {
                        success: false,
                        message: format!("File saved but couldn't be read: {}", e),
                        path: file_path.to_string_lossy().to_string(),
                        exists,
                        writable,
                    })
                }
            }
        },
        Err(e) => {
            println!("File persistence error: {}", e);
            
            // Try to diagnose the issue
            let mut diagnostic = String::new();
            
            if let Ok(output) = std::process::Command::new("ls").args(&["-la", "/app/temp"]).output() {
                diagnostic.push_str(&format!("Directory listing: {}\n", 
                                           String::from_utf8_lossy(&output.stdout)));
            }
            
            if let Ok(output) = std::process::Command::new("df").args(&["-h"]).output() {
                diagnostic.push_str(&format!("Disk space: {}\n", 
                                           String::from_utf8_lossy(&output.stdout)));
            }
            
            Json(UploadResponse {
                success: false,
                message: format!("Failed to save file: {}. Diagnostics: {}", e, diagnostic),
                path: file_path.to_string_lossy().to_string(),
                exists,
                writable,
            })
        }
    }
}

#[get("/")]
fn index() -> &'static str {
    "Minimal Upload Test Server"
}

#[launch]
fn rocket() -> _ {
    let figment = rocket::Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge(("port", 8000))
        .merge(("log_level", "debug"));

    rocket::custom(figment)
        .mount("/", routes![index, debug_upload])
}
