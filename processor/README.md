# Video Processor Setup

This is a standalone Rust application that runs on a VPS (like EC2, Railway, Fly.io) or your local machine. It uses FFmpeg to process videos.

## 1. Prerequisites
- **Rust** installed.
- **FFmpeg** installed (`brew install ffmpeg` or `apt install ffmpeg`).
- **R2 Keys**: Access Key ID and Secret Access Key (from Cloudflare Dashboard).

## 2. Configuration (`.env`)
Create a `.env` file in this directory:

```env
RUST_LOG=info
PORT=8080
R2_ACCESS_KEY_ID=your_access_key
R2_SECRET_ACCESS_KEY=your_secret_key
R2_BUCKET_NAME=copy-right
R2_ENDPOINT=https://your-account-id.r2.cloudflarestorage.com
```

## 3. Build & Run
```bash
cargo run --release
```

## 4. API Usage
The `upload-api` worker will call this service independently.
- **POST /process**: Triggers processing for a video.



Uploaded video-upload-api (8.15 sec)
Deployed video-upload-api triggers (5.16 sec)
  https://video-upload-api.pripritam7.workers.dev
Current Version ID: 1121a8ef-27f4-4298-a20d-6e94f14df011