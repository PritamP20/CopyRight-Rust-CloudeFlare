use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber;

mod audio;
mod download;
mod fingerprint;

#[derive(Debug, Deserialize)]
struct ProcessRequest {
    video_id: String,
    r2_key: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok(); // Load .env file
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Check if API URL is set, for info logging only (we read it in handler)
    let api_url =
        std::env::var("UPLOAD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8787".to_string());
    tracing::info!("Configured to callback Upload API at: {}", api_url);

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

    match download::download_video(&payload.video_id, &payload.r2_key).await {
        Ok(path) => {
            tracing::info!("Video downloaded to: {:?}", path);

            // Process fingerprints
            let video_hashes_result = fingerprint::process_video(&path).await;

            // Process Audio
            let audio_hashes = match audio::process_audio(&path).await {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!("Audio processing failed: {}", e);
                    Vec::new()
                }
            };

            match video_hashes_result {
                Ok(hashes) => {
                    tracing::info!(
                        "Generated {} video hashes, {} audio hashes",
                        hashes.len(),
                        audio_hashes.len()
                    );

                    // CALL WORKER API TO CHECK DUPLICATES & STORE
                    let client = Client::new();
                    let api_url = std::env::var("UPLOAD_API_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:8787".to_string());
                    let target_url = format!("{}/internal/complete", api_url);

                    let body = json!({
                        "video_id": payload.video_id,
                        "hashes": hashes,
                        "audio_hashes": audio_hashes
                    });

                    match client.post(&target_url).json(&body).send().await {
                        Ok(res) => {
                            if res.status() == StatusCode::OK {
                                tracing::info!("Video indexed successfully via API");
                                (
                                    StatusCode::OK,
                                    format!("Processed and indexed video {}", payload.video_id),
                                )
                            } else if res.status() == StatusCode::CONFLICT {
                                tracing::warn!("Duplicate detected by API!");
                                (
                                    StatusCode::CONFLICT,
                                    format!(
                                        "Duplicate content detected for video {}",
                                        payload.video_id
                                    ),
                                )
                            } else {
                                let err_text = res.text().await.unwrap_or_default();
                                tracing::error!("API returned error: {}", err_text);
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("Upload API Error: {}", err_text),
                                )
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to call Upload API: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Failed to contact Upload API: {}", e),
                            )
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to generate fingerprints: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to fingerprint video: {}", e),
                    )
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to download video: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to process video: {}", e),
            )
        }
    }
}
