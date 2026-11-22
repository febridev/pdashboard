mod config;
mod models;
mod services;

use axum::{routing::get, Router};
use config::Config;
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
    let _ = outline_client;
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }));

    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
