use axum::{extract::Json, http::StatusCode, response::IntoResponse, routing::post, Router};

mod download;
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber;

#[derive(Debug, Deserialize)]
struct ProcessRequest {
    video_id: String,
    r2_key: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let app = Router::new()
        .route("/process", post(process_video))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Processor service running on http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn process_video(Json(payload): Json<ProcessRequest>) -> impl IntoResponse {
    tracing::info!("Processing video: {:?}", payload);

    (StatusCode::OK, "Video processed successfully");
}
