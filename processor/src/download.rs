use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::{endpoint, Client, Config};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[tokio::main]
pub async fn download_video(
    video_id: &str,
    r2_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // y do we do Box

    dotenv::dotenv().ok();
    let access_key = env::var("R2_ACCESS_KEY_ID")?;
    let secret_key = env::var("R2_SECRET_ACCESS_KEY")?;
    let bucket = env::var("R2_BUCKET_NAME")?;
    let endpoint = env::var("R2_ENDPOINT")?;
    println!(
        "Simulating download for video_id: {}, key: {}",
        video_id, r2_key
    );
    Ok(())
}
