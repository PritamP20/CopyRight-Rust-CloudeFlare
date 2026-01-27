use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{Client, Config};
use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // what is the Box<dyn std::error::Error>>
    dotenv().ok();

    let endpoint_url = env::var("R2_ENDPOINT")?;
    let access_key = env::var("R2_ACCESS_KEY_ID")?;
    let secret_key = env::var("R2_SECRET_ACCESS_KEY")?;
    let bucket = env::var("R2_BUCKET_NAME")?;

    let creds = Credentials::new(access_key, secret_key, None, None, "r2");
    let config = Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("auto"))
        .endpoint_url(endpoint_url)
        .credentials_provider(creds)
        .build();

    let client = Client::from_conf(config);

    let files_bytes = tokio::fs::read("test.mp4").await?;
    let body = ByteStream::from(files_bytes);

    client
        .put_object()
        .bucket(&bucket)
        .key("test.mp4")
        .body(body)
        .send()
        .await?;

    println!("File uploaded successfully!");

    let objects = client.list_objects_v2().bucket(&bucket).send().await?;
    for obj in objects.contents() {
        println!("{}", obj.key().unwrap_or_default());
    }

    Ok(())
}
