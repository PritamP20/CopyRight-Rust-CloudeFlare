# Video Processor Implementation Guide

This guide details how to build the "heavy lifter" of the platform. This Rust service will run on a VPS (like EC2) and handle video analysis.

## 1. Project Setup

**Step 1:** Initialize the project (if you haven't)
```bash
cargo init
```

**Step 2:** Add Dependencies (`Cargo.toml`)
You need these creates for web server, async runtime, video processing, and s3 access.
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
axum = "0.7"              # Web server framework
tower-http = { version = "0.5", features = ["cors", "trace"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
aws-sdk-s3 = "1.0"        # For connecting to Cloudflare R2
image = "0.24"            # For image manipulation (resizing, grayscale)
ffmpeg-next = "6.0"       # Safe Rust bindings for FFmpeg
# OR execute ffmpeg via command line (simpler):
std = "0.50"
```

## 2. Implementation Steps

### Phase 1: The Web Server (`src/main.rs`)
Create a simple API that `upload-api` can call.

1.  Set up an `axum` router with a POST endpoint `/process`.
2.  Define a struct `ProcessRequest` to accept `{ "video_id": "...", "r2_key": "..." }`.
3.  Start the server on port 8080.

### Phase 2: Download Video
Interact with Cloudflare R2 to stream the video.

1.  Configure `aws-sdk-s3` with your R2 credentials (from `.env`).
2.  Use `get_object` to fetch the video stream using the `r2_key`.
3.  Save the video to a temporary file (e.g., `/tmp/video_id.mp4`).

### Phase 3: Video Fingerprinting (The Algorithm)
This detects if the video looks the same.

1.  **Extract Frames**: Use `ffmpeg` to extract 1 frame every second.
    ```bash
    ffmpeg -i input.mp4 -vf fps=1 frames/out%d.jpg
    ```
2.  **Process Each Frame**:
    *   **Resize**: Downscale to 32x32 pixels (removes detail, keeps structure).
    *   **Grayscale**: Convert to black & white (color changes shouldn't matter).
    *   **DCT (Discrete Cosine Transform)**: (Optional but recommended) Converts image to frequency domain.
    *   **Calculate Hash**: Compare each pixel to the mean value. 1 if >, 0 if <.
3.  **Combine**: Merge all frame hashes into a single vector.

### Phase 4: Audio Fingerprinting (Shazam-style)
This detects if the audio is the same.

1.  **Extract Audio**:
    ```bash
    ffmpeg -i input.mp4 -vn -ac 1 -ar 11025 audio.wav
    ```
2.  **Generate Fingerprint**: Use `fpcalc` (Chromaprint) on the wav file.
    ```rust
    // Run command line tool
    let output = Command::new("fpcalc").arg("audio.wav").output()?;
    ```
3.  The output is a long string representing the audio characteristics.

### Phase 5: Storage
Save the results.

1.  Connect to your Database (Cloudflare Vectorize or D1).
2.  Store the **Video Vector** (from Phase 3) and **Audio Hash** (from Phase 4) associated with the `video_id`.

## 3. Running It
Ensure FFmpeg is installed on your system.
```bash
brew install ffmpeg chromaprint # macOS
sudo apt install ffmpeg libchromaprint-tools # Linux
cargo run --release
```