use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub async fn extract_frames(video_path: &Path, temp_dir: &Path) -> Result<Vec<PathBuf>> {
    let output_pattern = temp_dir.join("frame_%04d.jpg");

    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_path)
        .arg("-vf")
        .arg("fps=1")
        .arg(&output_pattern)
        .status()
        .await
        .context("Failed to execute ffmpeg")?;

    if !status.success() {
        return Err(anyhow::anyhow!("FFmpeg exited with non-zero status"));
    }

    let mut frames = Vec::new();
    let mut read_dir = tokio::fs::read_dir(temp_dir).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jpg") {
            frames.push(path);
        }
    }

    frames.sort();

    Ok(frames)
}
