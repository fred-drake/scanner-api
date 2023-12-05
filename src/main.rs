use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

use tracing::{error, info};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[derive(Debug)]
struct AppState {
    child_process: Arc<Mutex<Option<Child>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    running: bool,
}

impl Status {
    fn new(running: bool) -> Status {
        Status { running }
    }
}

#[tracing::instrument]
async fn scan_endpoint(data: Data<AppState>) -> HttpResponse {
    if is_process_running(&data) {
        return HttpResponse::BadRequest().body("Child process already running");
    }

    let mut guard = data.child_process.lock().unwrap();

    if guard.is_none() {
        // Spawn the child process
        info!("Spawning scan process...");
        return if cfg!(test) {
            match Command::new("sleep").args(["15"]).spawn() {
                Ok(child) => {
                    *guard = Some(child);
                    HttpResponse::Ok().body("Child process started")
                }
                Err(e) => {
                    error!("Failure starting scan process: {}", e);
                    HttpResponse::InternalServerError().body("Failed to start child process")
                }
            }
        } else {
            match Command::new("epsonscan2")
                .args(["-s", "ES-400", "UserSettings.SF2"])
                .spawn()
            {
                Ok(child) => {
                    *guard = Some(child);
                    HttpResponse::Ok().body("Child process started")
                }
                Err(e) => {
                    error!("Failure starting scan process: {}", e);
                    HttpResponse::InternalServerError().body("Failed to start child process")
                }
            }
        };
    }

    HttpResponse::InternalServerError().body("Process should have been empty but wasn't")
}

#[tracing::instrument]
async fn status_endpoint(data: Data<AppState>) -> HttpResponse {
    let status = Status::new(is_process_running(&data));
    HttpResponse::Ok().json(status)
}

fn is_process_running(data: &Data<AppState>) -> bool {
    let mut guard = data.child_process.lock().unwrap();
    if guard.is_some() {
        let exit_status_option = guard.as_mut().unwrap().try_wait().unwrap();
        if let Some(exit_status) = exit_status_option {
            info!(
                "Scan process found exited with code {:?}",
                exit_status.code()
            );
            *guard = None;
            return false;
        }

        true
    } else {
        false
    }
}

#[actix_web::main]
#[tracing::instrument]
async fn main() -> std::io::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    #[cfg(not(feature = "dhat-heap"))]
    tracing_subscriber::fmt::init();

    let child_process: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let app_state = Data::new(AppState {
        child_process: Arc::clone(&child_process),
    });

    info!("Starting web server on port 8080.");
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/scan", web::post().to(scan_endpoint))
            .route("/status", web::get().to(status_endpoint))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use crate::Status;
    use reqwest::Response;

    #[actix_rt::test]
    async fn test_app() {
        let child_process: std::sync::Arc<std::sync::Mutex<Option<std::process::Child>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let app_state = actix_web::web::Data::new(crate::AppState {
            child_process: std::sync::Arc::clone(&child_process),
        });

        // Start the server in the background
        let srv = actix_web::HttpServer::new(move || {
            actix_web::App::new()
                .app_data(app_state.clone())
                .route("/scan", actix_web::web::post().to(crate::scan_endpoint))
                .route("/status", actix_web::web::get().to(crate::status_endpoint))
        })
        .bind("localhost:8080")
        .unwrap()
        .run();
        let srv_handle = srv.handle();
        actix_web::rt::spawn(srv);

        // Calling /status should return a 200 with running=false
        assert!(!status_call().await.running);

        // Calling /scan should return a 200 and kick off the scan process
        assert!(scan_call().await.status().is_success());

        // Calling /status should return a 200 with running=true
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(status_call().await.running);

        // Calling /scan again should return a 400
        assert!(scan_call().await.status().is_client_error());

        // After waiting for the scan to complete, /status should return a 200 with running=false
        std::thread::sleep(std::time::Duration::from_secs(16));
        assert!(!status_call().await.running);

        srv_handle.stop(false).await;
    }

    async fn status_call() -> Status {
        let response = reqwest::get("http://localhost:8080/status").await.unwrap();
        return serde_json::from_str(response.text().await.unwrap().as_str()).unwrap();
    }

    async fn scan_call() -> Response {
        let client = reqwest::Client::new();
        client
            .post("http://localhost:8080/scan")
            .send()
            .await
            .unwrap()
    }
}
