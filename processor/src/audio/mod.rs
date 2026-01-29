pub mod extract;
pub mod shazam;

use anyhow::Result;
use std::path::Path;

pub async fn process_audio(video_path: &Path) -> Result<Vec<shazam::AudioHash>> {
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    let audio_path = extract::extract_audio(video_path, temp_path).await?;
    tracing::info!("Extracted audio to {:?}", audio_path);
    let hashes = shazam::compute_audio_fingerprints(&audio_path)?;

    tracing::info!("Generated {} audio hashes", hashes.len());
    Ok(hashes)
}
