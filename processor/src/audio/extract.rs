use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub async fn extract_audio(video_path: &Path, temp_dir: &Path) -> Result<PathBuf> {
    let output_path = temp_dir.join("audio.wav");

    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_path)
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("44100")
        .arg("-y")
        .arg(&output_path)
        .status()
        .await
        .context("Failed to execute ffmpeg for audio extraction")?;

    if !status.success() {
        return Err(anyhow::anyhow!("ffmpeg failed to extract audio"));
    }

    Ok(output_path)
}
