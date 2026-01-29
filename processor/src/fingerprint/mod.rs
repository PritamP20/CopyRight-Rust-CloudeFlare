pub mod extract;
pub mod hash;

use anyhow::Result;
use img_hash::ImageHash;
use std::path::Path;

pub async fn process_video(video_path: &Path) -> Result<Vec<String>> {
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    let frames = extract::extract_frames(video_path, temp_path).await?;
    tracing::info!("Extracted {} frames", frames.len());

    let mut hashes = Vec::new();
    for frame in frames {
        let hash = hash::compute_phash(&frame)?;
        let bytes = hash.as_bytes();
        let hex_str: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        hashes.push(hex_str);
    }

    Ok(hashes)
}

mod tests;
