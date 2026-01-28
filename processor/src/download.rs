use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::{Client, Config};
use std::env;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

use anyhow::Result;

pub async fn download_video(video_id: &str, r2_key: &str) -> Result<PathBuf> {
    println!("Starting download for video_id: {}", video_id);

    dotenv::dotenv().ok();
    let access_key = env::var("R2_ACCESS_KEY_ID")?;
    let secret_key = env::var("R2_SECRET_ACCESS_KEY")?;
    let bucket = env::var("R2_BUCKET_NAME")?;
    let account_id = env::var("R2_ACCOUNT_ID")?;

    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);
    let creds = Credentials::new(access_key, secret_key, None, None, "r2");
    let config = Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("auto"))
        .endpoint_url(endpoint_url)
        .credentials_provider(creds)
        .build();

    let client = Client::from_conf(config);

    let mut response = client
        .get_object()
        .bucket(&bucket)
        .key(r2_key)
        .send()
        .await?;

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("{}.mp4", video_id));

    let mut file = tokio::fs::File::create(&file_path).await?;

    while let Some(bytes) = response.body.try_next().await? {
        file.write_all(&bytes).await?;
    }

    file.flush().await?;

    println!("Downloaded video to: {:?}", file_path);
    Ok(file_path)
}
