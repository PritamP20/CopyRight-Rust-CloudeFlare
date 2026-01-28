pub mod extract;
pub mod hash;

use anyhow::Result;
use std::path::Path;

pub async fn process_video(video_path: &Path) -> Result<Vec<String>> {
    // Create a temporary directory for frames
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    // 1. Extract frames
    let frames = extract::extract_frames(video_path, temp_path).await?;
    tracing::info!("Extracted {} frames", frames.len());

    // 2. Compute hashes
    let mut hashes = Vec::new();
    for frame in frames {
        // Hashing is CPU intensive, strictly blocking, so we might want to spawn_blocking
        // explicitly if we were doing many in parallel. For now, simple loop is fine.
        let hash = hash::compute_phash(&frame)?;
        hashes.push(hash);
    }

    Ok(hashes)
}
