mod config;
mod models;
mod routes;
mod services;

use axum::{routing::get, Router};
use config::Config;
use routes::tasks::{get_tasks, AppState};
use services::outline::OutlineClient;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let config = Config::from_env();
    let config = Arc::new(config);
    let outline_client = OutlineClient::new(config.clone());

    let state = AppState {
        outline_client,
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/tasks", get(get_tasks))
        .with_state(state);

    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
