#[cfg(test)]
mod tests {
    use crate::fingerprint::process_video;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_process_video() {
        let video_path = PathBuf::from("test_video.mp4");

        if !video_path.exists() {
            eprintln!("Skipping test: test_video.mp4 not found. Run 'ffmpeg -f lavfi -i \"smptebars=size=1280x720:duration=5\" -c:v libx264 -pix_fmt yuv420p test_video.mp4' to generate it.");
            return;
        }

        let result = process_video(&video_path).await;
        assert!(
            result.is_ok(),
            "Failed to process video: {:?}",
            result.err()
        );

        let hashes = result.unwrap();
        assert!(!hashes.is_empty(), "No fingerprint hashes generated");
        println!("Generated {} hashes: {:?}", hashes.len(), hashes);
    }
}
