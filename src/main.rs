mod aws;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use rand::Rng;
use serde_json::json;
use git2::Repository;
use std::fs;
use std::path::Path;
use tokio::task;
use aws::S3Uploader;
use rusoto_core::Region;

#[derive(Deserialize)]
struct Repourl {
    url: String,
}

#[get("/")]
async fn hello() -> impl Responder {
    "Hello World!"
}

async fn clone_repo_async(url: String, id: String) -> Result<(), String> {
    task::spawn_blocking(move || {
        let output_folder = format!("output/{}", id);
        let path = Path::new(&output_folder);

        // Create the output directory
        if let Err(e) = fs::create_dir_all(&output_folder) {
            return Err(format!("Failed to create directory: {}", e));
        }

        // Clone the repository
        match Repository::clone(&url, &output_folder) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Remove the directory if cloning fails
                let _ = fs::remove_dir_all(&output_folder);
                Err(format!("Failed to clone repository: {}", e))
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn upload_to_s3(id: String) -> Result<(), String> {
    let folder_path = format!("output/{}", id);
    let s3_uploader = S3Uploader::new("aws4sohan".to_string(), Region::ApSouth1);

    s3_uploader
        .upload_folder(id, Path::new(&folder_path))
        .await
        .map_err(|e| e.to_string())
}

#[post("/submit")]
async fn submit(item: web::Json<Repourl>) -> HttpResponse {
    let id = generate_id();
    match clone_repo_async(item.url.clone(), id.clone()).await {
        Ok(_) => match upload_to_s3(id.clone()).await {
            Ok(_) => {
                let response = json!({ "id": id });
                HttpResponse::Ok().json(response)
            }
            Err(e) => {
                let response = json!({ "error": e });
                HttpResponse::Ok().json(response)
            }
        },
        Err(e) => {
            let response = json!({ "error": e });
            HttpResponse::Ok().json(response)
        }
    }
}

fn generate_id() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    const ID_LENGTH: usize = 5;
    let mut rng = rand::thread_rng();

    (0..ID_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(hello)
            .service(submit)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
