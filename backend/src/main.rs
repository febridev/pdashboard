mod config;
mod models;
mod routes;
mod services;

use axum::{routing::get, Router};
use config::Config;
use routes::tasks::{get_tasks, AppState};
use routes::auth::{google_login, google_callback, logout};
use services::auth::AuthService;
use services::live::{spawn_poller, TaskEvents};
use services::outline::OutlineClient;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let config = Config::from_env();
    let config = Arc::new(config);
    let outline_client = OutlineClient::new(config.clone());
    let auth_service = Arc::new(AuthService::new(config.clone()));
    let task_events = Arc::new(TaskEvents::new());

    // Background poller: refreshes the task snapshot from Outline and
    // publishes it to SSE subscribers when it changes.
    spawn_poller(
        outline_client.clone(),
        task_events.clone(),
        Duration::from_secs(config.task_poll_interval_secs),
    );

    let state = AppState {
        outline_client,
        auth_service,
        csrf_states: Arc::new(Mutex::new(HashMap::new())),
        sessions: Arc::new(Mutex::new(HashMap::new())),
        task_events,
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/tasks", get(get_tasks))
        .route("/auth/google", get(google_login))
        .route("/auth/google/callback", get(google_callback))
        .route("/auth/logout", get(logout))
        .with_state(state);

    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
