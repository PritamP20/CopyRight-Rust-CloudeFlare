# Video Duplicate Detection Platform - Complete Documentation

## 📋 Table of Contents

1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [How It Works](#how-it-works)
4. [Technology Stack](#technology-stack)
5. [Data Flow](#data-flow)
6. [Fingerprinting Algorithm](#fingerprinting-algorithm)
7. [Duplicate Detection Logic](#duplicate-detection-logic)
8. [Database Schema](#database-schema)
9. [API Endpoints](#api-endpoints)
10. [Project Structure](#project-structure)
11. [Setup & Installation](#setup--installation)
12. [Configuration](#configuration)
13. [Deployment](#deployment)
14. [Performance & Scaling](#performance--scaling)
15. [Cost Analysis](#cost-analysis)
16. [Monitoring & Logging](#monitoring--logging)
17. [Troubleshooting](#troubleshooting)
18. [Learning Resources](#learning-resources)

---

## Overview

A scalable video platform built with Rust and Cloudflare that automatically detects duplicate/copied videos using perceptual hashing and audio fingerprinting - similar to YouTube's Content ID system.

### Key Features

- ✅ **Automatic Duplicate Detection**: Identifies copied videos even with modifications
- ✅ **Scalable Architecture**: Handles millions of videos efficiently
- ✅ **Fast Processing**: Async queue-based fingerprinting
- ✅ **Cost Effective**: Uses Cloudflare's edge infrastructure
- ✅ **Robust Detection**: Detects copies despite re-encoding, cropping, watermarks
- ✅ **Real-time API**: RESTful API built with Rust Workers

### What It Detects

The system can identify duplicate videos even when:
- Re-encoded with different codec (H.264 → H.265)
- Resolution changed (1080p → 720p)
- Brightness/contrast adjusted
- Cropped or letterboxed
- Watermarks added
- Slightly trimmed
- Audio pitch shifted

---

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         USER LAYER                              │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐               │
│  │ Web App  │     │ Mobile   │     │  API     │               │
│  │          │     │   App    │     │ Clients  │               │
│  └────┬─────┘     └────┬─────┘     └────┬─────┘               │
└───────┼────────────────┼────────────────┼─────────────────────┘
        │                │                │
        └────────────────┼────────────────┘
                         │ HTTPS
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CLOUDFLARE EDGE NETWORK                      │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              Upload API Worker (Rust)                      │ │
│  │  • Validate video (format, size, duration)                │ │
│  │  • Generate unique video ID (UUID)                        │ │
│  │  • Upload video to R2 bucket                              │ │
│  │  • Insert metadata to D1 database                         │ │
│  │  • Queue processing job                                   │ │
│  │  • Return video_id to client                              │ │
│  └─────────────────────┬─────────────────────────────────────┘ │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              Cloudflare Queue                              │ │
│  │  Job Schema: {                                            │ │
│  │    video_id: "uuid",                                      │ │
│  │    r2_key: "videos/2024/01/27/uuid.mp4",                 │ │
│  │    user_id: "user_uuid"                                   │ │
│  │  }                                                         │ │
│  └─────────────────────┬─────────────────────────────────────┘ │
│                        │                                         │
│                        ▼                                         │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │         Queue Consumer Worker (Rust)                       │ │
│  │  1. Receive job from queue                                │ │
│  │  2. Call external processor with R2 URL                   │ │
│  │  3. Receive fingerprint response                          │ │
│  │  4. Query Vectorize for similar videos                    │ │
│  │  5. Make duplicate/unique decision                        │ │
│  │  6. Update D1 database                                    │ │
│  │  7. Store fingerprint in Vectorize (if unique)            │ │
│  └─────────────────────┬─────────────────────────────────────┘ │
└────────────────────────┼─────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              EXTERNAL PROCESSOR (Your Server)                   │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │         Rust Video Processing Service                      │ │
│  │                                                            │ │
│  │  Video Fingerprinting:                                    │ │
│  │  1. Download video from R2 (streaming)                    │ │
│  │  2. Extract keyframes (ffmpeg)                            │ │
│  │     - Every 1-2 seconds                                   │ │
│  │     - Or only I-frames (keyframes)                        │ │
│  │  3. For each frame:                                       │ │
│  │     a. Resize to 32x32 pixels                             │ │
│  │     b. Convert to grayscale                               │ │
│  │     c. Apply DCT (Discrete Cosine Transform)              │ │
│  │     d. Extract low-frequency coefficients                 │ │
│  │     e. Generate perceptual hash (pHash)                   │ │
│  │  4. Combine frame hashes into vector                      │ │
│  │                                                            │ │
│  │  Audio Fingerprinting:                                    │ │
│  │  1. Extract audio track                                   │ │
│  │  2. Use Chromaprint algorithm                             │ │
│  │  3. Generate acoustic fingerprint                         │ │
│  │                                                            │ │
│  │  Return: {                                                │ │
│  │    video_vector: [0.2, 0.5, 0.8, ...],  // 64-128 dims   │ │
│  │    audio_hash: "AQADtE...",                               │ │
│  │    duration: 125,                                         │ │
│  │    frame_count: 62                                        │ │
│  │  }                                                         │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CLOUDFLARE STORAGE                           │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │      R2      │  │  Vectorize   │  │      D1      │         │
│  │   (Videos)   │  │ (Fingerprints│  │  (Metadata)  │         │
│  │              │  │   Vectors)   │  │              │         │
│  │ • Raw video  │  │ • 64-128 dim │  │ • video_id   │         │
│  │   files      │  │   vectors    │  │ • r2_key     │         │
│  │ • Thumbnails │  │ • Cosine     │  │ • status     │         │
│  │ • Segments   │  │   similarity │  │ • duplicate_ │         │
│  │              │  │ • ANN search │  │   of         │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### 1. **Upload API Worker (Cloudflare Worker - Rust)**
- **Runtime**: Cloudflare Workers (Edge compute)
- **Purpose**: Handle video uploads, validation, initial storage
- **CPU Limit**: 50ms-30s depending on plan
- **Memory**: 128MB

**Responsibilities**:
```
✓ Validate file format (mp4, webm, mov)
✓ Check file size (max 2GB per upload)
✓ Virus scanning (optional via Cloudflare Gateway)
✓ Generate UUID for video
✓ Upload to R2 with organized path structure
✓ Insert initial record to D1
✓ Queue fingerprinting job
✓ Return immediate response to user
```

#### 2. **Queue Consumer Worker (Cloudflare Worker - Rust)**
- **Runtime**: Cloudflare Workers (Triggered by Queue)
- **Purpose**: Orchestrate fingerprinting and duplicate detection
- **Trigger**: Cloudflare Queue message
- **Concurrency**: Up to 100 concurrent executions

**Responsibilities**:
```
✓ Receive jobs from queue
✓ Call external processor API
✓ Handle retry logic for failed processing
✓ Query Vectorize for similar videos
✓ Calculate similarity scores
✓ Make duplicate/unique decision
✓ Update database status
✓ Store new fingerprints
✓ Send notifications (optional)
```

#### 3. **External Processor (Your Server - Rust)**
- **Runtime**: Dedicated server (not Cloudflare)
- **Purpose**: Heavy video processing (CPU/GPU intensive)
- **Recommended**: AWS EC2, Railway, Fly.io, or your own server
- **Resources**: 4+ CPU cores, 8GB+ RAM

**Why External?**
- Cloudflare Workers have CPU time limits (30s max)
- FFmpeg processing can take 30s-5min per video
- Need access to GPU for faster processing (optional)
- Can scale independently

**Responsibilities**:
```
✓ Download video from R2
✓ Extract frames using FFmpeg
✓ Generate perceptual hashes (pHash)
✓ Extract audio fingerprints (Chromaprint)
✓ Create feature vectors
✓ Return compact fingerprint data
```

#### 4. **Cloudflare R2 (Object Storage)**
- **Purpose**: Store all video files
- **Structure**:
  ```
  videos/
    2024/
      01/
        27/
          {video_id}.mp4
          {video_id}_thumb.jpg
    processed/
      {video_id}/
        720p.mp4
        1080p.mp4
  ```

#### 5. **Cloudflare Vectorize (Vector Database)**
- **Purpose**: Store and search video fingerprints
- **Index Configuration**:
  ```
  dimensions: 64-128 (depending on your pHash implementation)
  metric: cosine (similarity measure)
  ```
- **Performance**: Sub-100ms searches even with millions of vectors

#### 6. **Cloudflare D1 (SQLite Database)**
- **Purpose**: Store video metadata and relationships
- **Size**: Unlimited rows, pay per read/write
- **Query Performance**: <10ms for indexed queries

---

## How It Works

### Phase 1: Video Upload (Synchronous - <1 second)

```
┌─────────┐
│  User   │
└────┬────┘
     │ POST /api/upload
     │ Content-Type: multipart/form-data
     │ Body: video file
     ▼
┌─────────────────────────────────────┐
│    Upload API Worker                │
│                                     │
│  1. Parse multipart data            │
│     const form = req.formData()     │
│     const file = form.get("video")  │
│                                     │
│  2. Validate                        │
│     ✓ Size < 2GB                    │
│     ✓ Format: mp4/webm/mov          │
│     ✓ Duration < 1 hour (optional)  │
│                                     │
│  3. Generate ID                     │
│     video_id = UUID::new_v4()       │
│                                     │
│  4. Upload to R2                    │
│     r2_key = "videos/2024/01/27/    │
│              {video_id}.mp4"        │
│     r2.put(r2_key, file_bytes)      │
│                                     │
│  5. Insert to D1                    │
│     INSERT INTO videos              │
│     (id, r2_key, user_id, status)   │
│     VALUES (?, ?, ?, 'processing')  │
│                                     │
│  6. Queue job                       │
│     queue.send({                    │
│       video_id,                     │
│       r2_key,                       │
│       user_id                       │
│     })                              │
│                                     │
│  7. Return response                 │
│     200 OK                          │
│     {                               │
│       "video_id": "abc-123",        │
│       "status": "processing"        │
│     }                               │
└─────────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────┐
│   Cloudflare Queue                  │
│   [Job queued for processing]       │
└─────────────────────────────────────┘
```

**Timeline**: ~200-800ms total

### Phase 2: Fingerprint Extraction (Asynchronous - 10s-2min)

```
┌─────────────────────────────────────┐
│  Cloudflare Queue                   │
│  Triggers consumer every 1s or      │
│  when batch size reached (10 jobs)  │
└────┬────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────┐
│  Queue Consumer Worker              │
│                                     │
│  Receives batch of jobs:            │
│  [                                  │
│    {video_id: "abc", r2_key: "..."} │
│    {video_id: "def", r2_key: "..."} │
│  ]                                  │
│                                     │
│  For each job:                      │
│                                     │
│  1. Get R2 presigned URL            │
│     let url = r2.createPresignedUrl(│
│       r2_key,                       │
│       expires_in: 3600              │
│     )                               │
│                                     │
│  2. Call processor                  │
│     POST https://processor.com/     │
│          extract-fingerprint        │
│     {                               │
│       "video_url": url,             │
│       "video_id": "abc-123"         │
│     }                               │
│                                     │
│  3. Wait for response (10s-120s)    │
│                                     │
└────┬────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────────────────────┐
│  External Processor                                 │
│                                                     │
│  Receives request                                   │
│                                                     │
│  STEP 1: Download Video                             │
│  ────────────────────────                           │
│  • Stream from R2 URL                               │
│  • Don't save full file to disk                     │
│  • Process in chunks                                │
│                                                     │
│  STEP 2: Extract Frames                             │
│  ────────────────────────                           │
│  ffmpeg -i video.mp4 \                              │
│    -vf "select='eq(pict_type,I)',                   │
│         fps=1/2" \                                  │
│    -vsync 0 \                                       │
│    frames/frame_%04d.jpg                            │
│                                                     │
│  Result: Extract 1 frame every 2 seconds            │
│  Example: 60s video → 30 frames                     │
│                                                     │
│  STEP 3: Generate pHash for Each Frame              │
│  ────────────────────────────────────               │
│  For each frame:                                    │
│    a. Load image                                    │
│    b. Resize to 32x32 pixels                        │
│       let resized = resize_image(                   │
│         frame, 32, 32                               │
│       )                                             │
│                                                     │
│    c. Convert to grayscale                          │
│       let gray = to_grayscale(resized)              │
│                                                     │
│    d. Apply DCT                                     │
│       // DCT converts spatial data to frequency     │
│       let dct_matrix = apply_dct(gray)              │
│                                                     │
│    e. Extract top-left 8x8 coefficients             │
│       // Low frequencies = general structure        │
│       let coeffs = dct_matrix[0..8][0..8]           │
│                                                     │
│    f. Flatten to vector                             │
│       let hash_vector = flatten(coeffs)             │
│       // Result: [0.234, 0.567, 0.891, ...]         │
│                                                     │
│  STEP 4: Combine Frame Hashes                       │
│  ────────────────────────────────                   │
│  // Average all frame vectors                       │
│  video_vector = average([                           │
│    frame1_hash,  // [0.2, 0.5, ...]                │
│    frame2_hash,  // [0.3, 0.4, ...]                │
│    frame3_hash,  // [0.1, 0.6, ...]                │
│  ])                                                 │
│  // Result: [0.2, 0.5, ...] (64 dimensions)         │
│                                                     │
│  STEP 5: Extract Audio Fingerprint                  │
│  ────────────────────────────────────               │
│  ffmpeg -i video.mp4 \                              │
│    -vn -ar 11025 -ac 1 \                            │
│    audio.wav                                        │
│                                                     │
│  chromaprint audio.wav                              │
│  // Uses acoustic landmarks                         │
│  // Similar to how Shazam works                     │
│                                                     │
│  Result: "AQADtEmUaEkSRYmS..."                      │
│                                                     │
│  STEP 6: Return Response                            │
│  ────────────────────────────                       │
│  {                                                  │
│    "video_vector": [0.2, 0.5, 0.8, ...],  // 64d   │
│    "audio_hash": "AQADtEmUaEkSRYmS...",             │
│    "duration": 125,                                 │
│    "frame_count": 62,                               │
│    "processing_time_ms": 8450                       │
│  }                                                  │
└────┬────────────────────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────┐
│  Queue Consumer (continues)         │
│                                     │
│  Received fingerprint response      │
└─────────────────────────────────────┘
```

**Timeline**: 10-120 seconds depending on video length

### Phase 3: Duplicate Detection (Fast - <1 second)

```
┌─────────────────────────────────────────────────────┐
│  Queue Consumer Worker                              │
│                                                     │
│  Has fingerprint:                                   │
│  {                                                  │
│    video_vector: [0.2, 0.5, 0.8, ...],             │
│    audio_hash: "AQADtEm..."                         │
│  }                                                  │
│                                                     │
│  STEP 1: Query Vectorize                            │
│  ─────────────────────                              │
│  vectorize.query(                                   │
│    vector: video_vector,                            │
│    top_k: 5,          // Get top 5 matches          │
│    threshold: 0.80    // Min 80% similarity         │
│  )                                                  │
│                                                     │
│  How Vectorize Works:                               │
│  ┌──────────────────────────────────┐              │
│  │  Cosine Similarity Formula:      │              │
│  │                                  │              │
│  │  similarity = (A · B) /          │              │
│  │               (||A|| × ||B||)    │              │
│  │                                  │              │
│  │  Where:                          │              │
│  │  A = new video vector            │              │
│  │  B = stored video vector         │              │
│  │  · = dot product                 │              │
│  │  || || = magnitude               │              │
│  │                                  │              │
│  │  Result: 0.0 to 1.0              │              │
│  │  0.0 = completely different      │              │
│  │  1.0 = identical                 │              │
│  └──────────────────────────────────┘              │
│                                                     │
│  Response from Vectorize:                           │
│  [                                                  │
│    {                                                │
│      id: "video_xyz",                               │
│      score: 0.92,     // 92% similar                │
│      metadata: {                                    │
│        r2_key: "...",                               │
│        audio_hash: "AQADtEm...",                    │
│        uploaded_at: "2024-01-20"                    │
│      }                                              │
│    },                                               │
│    {                                                │
│      id: "video_abc",                               │
│      score: 0.87,     // 87% similar                │
│      metadata: {...}                                │
│    }                                                │
│  ]                                                  │
│                                                     │
│  STEP 2: Analyze Results                            │
│  ─────────────────────                              │
│                                                     │
│  IF matches.is_empty():                             │
│    → Video is UNIQUE                                │
│    → Go to STEP 4                                   │
│                                                     │
│  ELSE:                                              │
│    best_match = matches[0]                          │
│                                                     │
│    IF best_match.score >= 0.85:                     │
│      → High confidence DUPLICATE                    │
│                                                     │
│      // Optional: Double-check audio                │
│      IF audio_hash_similar(                         │
│           new_audio,                                │
│           best_match.audio_hash                     │
│         ):                                          │
│        → CONFIRMED DUPLICATE                        │
│      ELSE:                                          │
│        → Possible false positive                    │
│        → Flag for manual review                     │
│                                                     │
│  STEP 3: Handle Duplicate                           │
│  ─────────────────────                              │
│  UPDATE videos                                      │
│  SET                                                │
│    status = 'duplicate',                            │
│    duplicate_of = 'video_xyz',                      │
│    similarity_score = 0.92,                         │
│    processed_at = NOW()                             │
│  WHERE id = 'new_video_id'                          │
│                                                     │
│  // Optional: Delete from R2 to save storage        │
│  r2.delete(r2_key)                                  │
│                                                     │
│  // Send notification                               │
│  notify_user({                                      │
│    type: "duplicate_detected",                      │
│    original_video: "video_xyz",                     │
│    similarity: 0.92                                 │
│  })                                                 │
│                                                     │
│  RETURN                                             │
│                                                     │
│  STEP 4: Store Unique Video                         │
│  ─────────────────────────                          │
│  // Insert fingerprint to Vectorize                 │
│  vectorize.insert(                                  │
│    id: new_video_id,                                │
│    vector: video_vector,                            │
│    metadata: {                                      │
│      r2_key,                                        │
│      audio_hash,                                    │
│      duration,                                      │
│      uploaded_at                                    │
│    }                                                │
│  )                                                  │
│                                                     │
│  // Update D1                                       │
│  UPDATE videos                                      │
│  SET                                                │
│    status = 'approved',                             │
│    fingerprint_id = vectorize_id,                   │
│    processed_at = NOW()                             │
│  WHERE id = new_video_id                            │
│                                                     │
│  // Send notification                               │
│  notify_user({                                      │
│    type: "video_approved",                          │
│    video_id: new_video_id                           │
│  })                                                 │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Timeline**: 50-500ms

### Complete Flow Timeline

```
Time  Event
────  ─────────────────────────────────────────
0:00  User uploads video
0:01  Upload complete, job queued
      ↓ (User gets response, can navigate away)
      ↓
0:02  Queue consumer picks up job
0:03  Processor starts downloading video
0:10  Processor extracts first frame
0:15  Processing frames (DCT, pHash)
0:45  Audio extraction starts
0:55  Fingerprint generation complete
0:56  Fingerprint sent back to worker
0:57  Vectorize similarity search
0:58  Decision: duplicate/unique
0:59  Database updated
1:00  User notification sent
      ✓ Processing complete
```

---

## Technology Stack

### Cloudflare Services

| Service | Purpose | Why We Use It |
|---------|---------|---------------|
| **Cloudflare Workers** | Rust-based serverless functions | Fast, edge-based compute, runs code globally |
| **Cloudflare R2** | Object storage | S3-compatible, zero egress fees, cheap storage |
| **Cloudflare Queues** | Message queue | Decouples upload from processing, handles retries |
| **Cloudflare D1** | SQLite database | Fast reads/writes, serverless SQL, automatic backups |
| **Cloudflare Vectorize** | Vector database | Optimized for similarity search, sub-100ms queries |

### Rust Dependencies

**Worker Dependencies:**
```toml
[dependencies]
worker = "0.4"              # Cloudflare Workers runtime
serde = "1.0"               # Serialization
serde_json = "1.0"          # JSON handling
uuid = { version = "1.0", features = ["v4"] }  # ID generation
chrono = "0.4"              # Date/time
reqwest = "0.12"            # HTTP client (for calling processor)
```

**Processor Dependencies:**
```toml
[dependencies]
actix-web = "4.0"           # Web framework
tokio = "1.0"               # Async runtime
ffmpeg-next = "7.0"         # FFmpeg bindings
image = "0.25"              # Image processing
rustdct = "0.7"             # DCT implementation
chromaprint = "0.1"         # Audio fingerprinting
aws-sdk-s3 = "1.0"          # R2 access (S3-compatible)
serde = "1.0"
serde_json = "1.0"
```

### External Tools

| Tool | Purpose | Installation |
|------|---------|--------------|
| **FFmpeg** | Video/audio processing | `apt install ffmpeg` |
| **Chromaprint** | Audio fingerprinting | `apt install libchromaprint-dev` |
| **fpcalc** | Chromaprint CLI | `apt install libchromaprint-tools` |

---

## Data Flow

### Upload Flow Diagram

```
┌─────────┐
│  User   │
└────┬────┘
     │
     │ 1. POST /api/upload
     │    multipart/form-data
     │    video: file.mp4
     │
     ▼
┌─────────────────────────────────────┐
│  Upload Worker                      │
│  ┌───────────────────────────────┐ │
│  │ Request Validation            │ │
│  │ • Check Content-Type          │ │
│  │ • Verify file size            │ │
│  │ • Validate user auth          │ │
│  └───────────────────────────────┘ │
│                │                    │
│                ▼                    │
│  ┌───────────────────────────────┐ │
│  │ File Processing               │ │
│  │ • Parse multipart data        │ │
│  │ • Extract video file          │ │
│  │ • Generate video_id (UUID)    │ │
│  └───────────────────────────────┘ │
│                │                    │
│                ▼                    │
│  ┌───────────────────────────────┐ │
│  │ R2 Upload                     │ │
│  │ PUT videos/2024/01/27/        │ │
│  │     {uuid}.mp4                │ │
│  │ • Stream upload (chunked)     │ │
│  │ • Set content-type            │ │
│  │ • Add metadata tags           │ │
│  └───────────────────────────────┘ │
│                │                    │
│                ▼                    │
│  ┌───────────────────────────────┐ │
│  │ D1 Insert                     │ │
│  │ INSERT INTO videos            │ │
│  │ VALUES (id, r2_key,           │ │
│  │         user_id, 'processing')│ │
│  └───────────────────────────────┘ │
│                │                    │
│                ▼                    │
│  ┌───────────────────────────────┐ │
│  │ Queue Job                     │ │
│  │ queue.send({                  │ │
│  │   video_id,                   │ │
│  │   r2_key,                     │ │
│  │   user_id                     │ │
│  │ })                            │ │
│  └───────────────────────────────┘ │
│                │                    │
└────────────────┼────────────────────┘
                 │
                 ▼
            ┌─────────┐
            │ Queue   │
            └─────────┘
                 │
                 ▼
            200 OK {
              "video_id": "...",
              "status": "processing"
            }
```

### Processing Flow Diagram

```
┌──────────────────┐
│ Cloudflare Queue │
└────────┬─────────┘
         │ Triggers consumer (1s interval or batch size)
         ▼
┌─────────────────────────────────────────────────────┐
│  Queue Consumer Worker                              │
│                                                     │
│  for job in batch {                                 │
│    ┌─────────────────────────────────────────┐    │
│    │ 1. Generate R2 Presigned URL            │    │
│    │    (temporary, expires in 1 hour)       │    │
│    └─────────────────────────────────────────┘    │
│                    │                                │
│                    ▼                                │
│    ┌─────────────────────────────────────────┐    │
│    │ 2. HTTP POST to Processor               │    │
│    │    POST /extract-fingerprint            │    │
│    │    {                                    │    │
│    │      "video_url": "https://r2...",      │    │
│    │      "video_id": "..."                  │    │
│    │    }                                    │    │
│    └─────────────────────────────────────────┘    │
│                    │                                │
│  }                 │                                │
└────────────────────┼────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────┐
│  External Processor                                 │
│                                                     │
│  ┌─────────────────────────────────────────┐      │
│  │ Video Download (Streaming)              │      │
│  │ GET https://r2.../video.mp4             │      │
│  └──────────────────┬──────────────────────┘      │
│                     ▼                               │
│  ┌─────────────────────────────────────────┐      │
│  │ Frame Extraction                         │      │
│  │ ffmpeg -i - -vf fps=1/2 frames/%04d.jpg │      │
│  │ → frame_0001.jpg                         │      │
│  │ → frame_0002.jpg                         │      │
│  │ → ...                                    │      │
│  └──────────────────┬──────────────────────┘      │
│                     ▼                               │
│  ┌─────────────────────────────────────────┐      │
│  │ pHash Generation                         │      │
│  │ For each frame:                          │      │
│  │   resize(32x32)                          │      │
│  │   → grayscale()                          │      │
│  │   → dct()                                │      │
│  │   → extract_coefficients()               │      │
│  │   → [0.2, 0.5, 0.8, ...]                │      │
│  └──────────────────┬──────────────────────┘      │
│                     ▼                               │
│  ┌─────────────────────────────────────────┐      │
│  │ Audio Fingerprint                        │      │
│  │ ffmpeg -i - -vn audio.wav                │      │
│  │ chromaprint → "AQADtEm..."               │      │
│  └──────────────────┬──────────────────────┘      │
│                     ▼                               │
│  ┌─────────────────────────────────────────┐      │
│  │ Return Response                          │      │
│  │ {                                        │      │
│  │   "video_vector": [...],                 │      │
│  │   "audio_hash": "...",                   │      │
│  │   "duration": 125                        │      │
│  │ }                                        │      │
│  └──────────────────┬──────────────────────┘      │
└─────────────────────┼──────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────┐
│  Queue Consumer (continues)                         │
│                                                     │
│  ┌─────────────────────────────────────────┐      │
│  │ 3. Query Vectorize                       │      │
│  │    vectorize.query(                      │      │
│  │      vector: video_vector,               │      │
│  │      top_k: 5,                           │      │
│  │      threshold: 0.85                     │      │
│  │    )                                     │      │
│  └──────────────────┬──────────────────────┘      │
│                     ▼                               │
│  ┌─────────────────────────────────────────┐      │
│  │ 4. Analyze Similarity                    │      │
│  │    IF best_match.score >= 0.85:          │      │
│  │      → DUPLICATE                         │      │
│  │    ELSE:                                 │      │
│  │      → UNIQUE                            │      │
│  └──────────────────┬──────────────────────┘      │
│                     ▼                               │
│           ┌─────────┴─────────┐                    │
│           │                   │                    │
│      DUPLICATE            UNIQUE                   │
│           │                   │                    │
│           ▼                   ▼                    │
│  ┌─────────────────┐  ┌──────────────────┐       │
│  │ Update D1       │  │ Insert Vectorize │       │
│  │ status='dup'    │  │ Store vector     │       │
│  │ duplicate_of=id │  │ Update D1        │       │
│  │ Delete from R2  │  │ status='approved'│       │
│  └─────────────────┘  └──────────────────┘       │
└─────────────────────────────────────────────────────┘
```

---

## Fingerprinting Algorithm

### Perceptual Hashing (pHash) Explained

**What is pHash?**
- A "fuzzy" hash that captures the visual essence of an image/video
- Similar images produce similar hashes (unlike MD5/SHA)
- Robust against minor modifications

**Step-by-Step Process:**

#### 1. **Resize to 32×32 pixels**

```
Original Frame: 1920×1080
         ↓
  [Resize Algorithm]
         ↓
  Resized: 32×32

Why 32×32?
• Removes fine details
• Keeps overall structure
• Computationally efficient
• Industry standard for pHash
```

#### 2. **Convert to Grayscale**

```
RGB Image: (R, G, B) per pixel
         ↓
  [Grayscale = 0.299R + 0.587G + 0.114B]
         ↓
Grayscale: Single value (0-255) per pixel

Why grayscale?
• Color variations don't matter for structure
• Reduces data by 3x
• Focus on luminance patterns
```

#### 3. **Apply DCT (Discrete Cosine Transform)**

```
Spatial Domain          Frequency Domain
┌─────────────┐        ┌─────────────┐
│ Pixel values│  DCT   │ Frequencies │
│ 234 156 189 │  ───→  │ Low   High  │
│ 201 145 167 │        │ freq  freq  │
│ 178 134 155 │        │             │
└─────────────┘        └─────────────┘

Low frequencies = general shapes, structure
High frequencies = details, noise, compression artifacts
```

**DCT Formula (simplified):**
```
F(u,v) = Σ Σ f(x,y) × cos((2x+1)uπ/2N) × cos((2y+1)vπ/2N)
         x y

Where:
- f(x,y) = pixel value at position (x,y)
- F(u,v) = DCT coefficient at frequency (u,v)
- N = 32 (our image size)
```

#### 4. **Extract Low-Frequency Coefficients**

```
Full DCT Matrix (32×32):
┌───────────────────────────┐
│ 180  45  12   3   1  ... │ ← Low freq (keep these)
│  52  23   8   2   0  ... │
│  18   9   3   1   0  ... │
│   5   2   1   0   0  ... │
│   1   0   0   0   0  ... │
│  ...                  ... │ ← High freq (discard)
└───────────────────────────┘
         ↓
Keep only top-left 8×8 = 64 values
         ↓
Flatten to vector: [180, 45, 12, ..., 0]
```

#### 5. **Normalize and Create Hash**

```
DCT Coefficients: [180, 45, 12, 3, 1, ...]
         ↓
Normalize to 0-1 range:
hash = coeffs / max(coeffs)
     = [1.0, 0.25, 0.067, 0.017, ...]
         ↓
This is your pHash!
```

### Audio Fingerprinting (Chromaprint)

**How Chromaprint Works:**

```
Audio Waveform
      ↓
1. Convert to spectrogram (time vs frequency)
      ↓
2. Identify "landmarks" (peaks in spectrum)
      ↓
3. Create fingerprint from landmark patterns
      ↓
Result: Compact hash string
```

**Process:**

```
Video → Extract Audio → FFmpeg converts to WAV
                              ↓
                    Sample at 11,025 Hz, mono
                              ↓
                    Chromaprint Analysis:
                    ┌────────────────────────┐
                    │ Time-Frequency Grid    │
                    │                        │
                    │  Freq  ●    ●    ●    │
                    │   │    ●  ● ●  ●  ●   │
                    │   │  ●  ●  ●  ●    ●  │
                    │   └──────────────────→ │
                    │         Time           │
                    └────────────────────────┘
                              ↓
                    Find Peaks (●)
                              ↓
                    Create Hash from Peak Patterns
                              ↓
                    "AQADtEmUaEkSRYmS..."
```

### Combining Video + Audio

```
Video pHash:  [0.2, 0.5, 0.8, 0.1, ...]  (64 dims)
Audio Hash:   "AQADtEmUaEkSRYmS..."       (string)

For duplicate detection:
1. First check video similarity (Vectorize query)
2. If close match (>85%), double-check audio
3. Both match → High confidence duplicate
4. Only one matches → Flag for review
```

---

## Duplicate Detection Logic

### Similarity Scoring

**Cosine Similarity Formula:**

```
Given two vectors A and B:

         A · B
cos(θ) = ─────────
         ||A|| ||B||

Where:
• A · B = dot product = Σ(Aᵢ × Bᵢ)
• ||A|| = magnitude = √(Σ Aᵢ²)
• Result: -1 to 1 (we use 0 to 1 for normalized vectors)
```

**Example Calculation:**

```
Video A hash: [0.8, 0.2, 0.5, 0.3]
Video B hash: [0.7, 0.3, 0.4, 0.4]

Dot product:
A · B = (0.8 × 0.7) + (0.2 × 0.3) + (0.5 × 0.4) + (0.3 × 0.4)
      = 0.56 + 0.06 + 0.20 + 0.12
      = 0.94

Magnitude A:
||A|| = √(0.8² + 0.2² + 0.5² + 0.3²)
      = √(0.64 + 0.04 + 0.25 + 0.09)
      = √1.02
      = 1.01

Magnitude B:
||B|| = √(0.7² + 0.3² + 0.4² + 0.4²)
      = √(0.49 + 0.09 + 0.16 + 0.16)
      = √0.90
      = 0.95

Cosine similarity:
cos(θ) = 0.94 / (1.01 × 0.95)
       = 0.94 / 0.96
       = 0.979
       ≈ 0.98 (98% similar!)
```

### Decision Thresholds

```
Similarity Score    Decision           Action
─────────────────   ────────────────   ──────────────────────
0.95 - 1.00         Exact duplicate    Auto-reject
0.85 - 0.94         High duplicate     Flag + review
0.70 - 0.84         Possible match     Allow + log
0.00 - 0.69         Unique             Approve
```

### Advanced Detection Logic

```rust
fn detect_duplicate(
    video_vector: Vec<f32>,
    audio_hash: String,
) -> DuplicateResult {
    // Step 1: Query Vectorize
    let matches = vectorize.query(
        video_vector,
        top_k: 5,
        threshold: 0.70  // Get anything >70%
    );
    
    if matches.is_empty() {
        return DuplicateResult::Unique;
    }
    
    let best_match = &matches[0];
    
    // Step 2: Video similarity check
    if best_match.score >= 0.95 {
        // Very high confidence - auto reject
        return DuplicateResult::Duplicate {
            original_id: best_match.id,
            confidence: "high",
            score: best_match.score,
        };
    }
    
    if best_match.score >= 0.85 {
        // High similarity - double check with audio
        let audio_similarity = compare_audio_hashes(
            &audio_hash,
            &best_match.metadata.audio_hash
        );
        
        if audio_similarity >= 0.80 {
            // Both video AND audio match - definitely duplicate
            return DuplicateResult::Duplicate {
                original_id: best_match.id,
                confidence: "very_high",
                score: (best_match.score + audio_similarity) / 2.0,
            };
        } else {
            // Video similar but audio different
            // Might be edited video or false positive
            return DuplicateResult::NeedsReview {
                original_id: best_match.id,
                reason: "video_match_audio_mismatch",
                video_score: best_match.score,
                audio_score: audio_similarity,
            };
        }
    }
    
    if best_match.score >= 0.70 {
        // Moderate similarity - log but allow
        return DuplicateResult::Unique {
            similar_to: Some(best_match.id),
            similarity: best_match.score,
        };
    }
    
    DuplicateResult::Unique
}
```

### Edge Cases Handled

| Scenario | Detection | Handling |
|----------|-----------|----------|
| **Trimmed video** | Video hash partially matches | Check if duration difference >20%, flag for review |
| **Mirrored/flipped** | Lower similarity (~70-80%) | If audio matches, likely duplicate |
| **Re-encoded** | High similarity (>90%) | Standard duplicate detection works |
| **Watermark added** | Slight similarity drop (~85-92%) | Still detects if watermark <10% of frame |
| **Color graded** | High similarity (>90%) | pHash focuses on structure, not color |
| **Cropped heavily** | Low similarity (<70%) | Won't detect - too different structurally |
| **Speed changed** | Video matches, audio different | Flag as "derivative work" |
| **Compilation** | Multiple partial matches | Special handling - not implemented in basic version |

---

## Database Schema

### D1 (SQLite) Schema

```sql
-- ============================================
-- VIDEOS TABLE
-- ============================================
-- Main table storing all video metadata
CREATE TABLE videos (
    -- Primary key
    id TEXT PRIMARY KEY,                    -- UUID v4
    
    -- Storage
    r2_key TEXT NOT NULL,                   -- Path in R2 bucket
    
    -- Ownership
    user_id TEXT NOT NULL,                  -- User who uploaded
    
    -- Status tracking
    status TEXT NOT NULL                    -- 'processing' | 'approved' | 'duplicate' | 'rejected' | 'flagged'
        CHECK(status IN ('processing', 'approved', 'duplicate', 'rejected', 'flagged')),
    
    -- Duplicate information
    duplicate_of TEXT,                      -- ID of original video (if duplicate)
    similarity_score REAL,                  -- Cosine similarity (0.0 - 1.0)
    
    -- Fingerprint reference
    fingerprint_id TEXT,                    -- ID in Vectorize
    
    -- Metadata
    duration INTEGER,                       -- Duration in seconds
    file_size INTEGER,                      -- Size in bytes
    mime_type TEXT,                         -- video/mp4, video/webm, etc.
    
    -- Timestamps
    uploaded_at TEXT NOT NULL,              -- ISO 8601 timestamp
    processed_at TEXT,                      -- When fingerprinting completed
    
    -- Foreign key (soft)
    FOREIGN KEY (duplicate_of) REFERENCES videos(id)
);

-- Indexes for fast queries
CREATE INDEX idx_videos_user_id ON videos(user_id);
CREATE INDEX idx_videos_status ON videos(status);
CREATE INDEX idx_videos_uploaded_at ON videos(uploaded_at);
CREATE INDEX idx_videos_duplicate_of ON videos(duplicate_of);

-- Composite index for common query pattern
CREATE INDEX idx_videos_user_status ON videos(user_id, status);

-- ============================================
-- PROCESSING_JOBS TABLE (Optional)
-- ============================================
-- Track processing attempts and failures
CREATE TABLE processing_jobs (
    id TEXT PRIMARY KEY,
    video_id TEXT NOT NULL,
    status TEXT NOT NULL 
        CHECK(status IN ('pending', 'processing', 'completed', 'failed')),
    attempt_count INTEGER DEFAULT 0,
    error_message TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    
    FOREIGN KEY (video_id) REFERENCES videos(id)
);

CREATE INDEX idx_jobs_video_id ON processing_jobs(video_id);
CREATE INDEX idx_jobs_status ON processing_jobs(status);

-- ============================================
-- USERS TABLE
-- ============================================
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    username TEXT UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT
);

CREATE INDEX idx_users_email ON users(email);

-- ============================================
-- AUDIT_LOG TABLE (Optional)
-- ============================================
-- Track all duplicate detections for analysis
CREATE TABLE duplicate_detections (
    id TEXT PRIMARY KEY,
    video_id TEXT NOT NULL,
    matched_video_id TEXT NOT NULL,
    similarity_score REAL NOT NULL,
    audio_match BOOLEAN,
    decision TEXT NOT NULL,  -- 'auto_reject' | 'flagged' | 'approved'
    detected_at TEXT NOT NULL,
    
    FOREIGN KEY (video_id) REFERENCES videos(id),
    FOREIGN KEY (matched_video_id) REFERENCES videos(id)
);

CREATE INDEX idx_detections_video ON duplicate_detections(video_id);
CREATE INDEX idx_detections_detected_at ON duplicate_detections(detected_at);
```

### Vectorize Schema

```javascript
// Vectorize Index Configuration
{
  "name": "video-fingerprints",
  "dimensions": 64,              // Size of pHash vector
  "metric": "cosine",            // Similarity metric
  "index_type": "auto"           // Let Vectorize choose optimal index
}

// Each vector entry:
{
  "id": "video_abc123",          // Same as video.id in D1
  "values": [0.2, 0.5, ...],     // 64-dimension pHash vector
  "metadata": {
    "r2_key": "videos/2024/...",
    "audio_hash": "AQADtEm...",
    "duration": 125,
    "uploaded_at": "2024-01-27T...",
    "user_id": "user_xyz"
  }
}
```

### Example Queries

```sql
-- Get all approved videos for a user
SELECT * FROM videos 
WHERE user_id = ?1 
  AND status = 'approved'
ORDER BY uploaded_at DESC
LIMIT 20;

-- Find all duplicates of a specific video
SELECT 
    v.*,
    u.username as uploader
FROM videos v
JOIN users u ON v.user_id = u.id
WHERE v.duplicate_of = ?1
ORDER BY v.uploaded_at DESC;

-- Get processing statistics
SELECT 
    status,
    COUNT(*) as count,
    AVG(
        CAST(
            (julianday(processed_at) - julianday(uploaded_at)) * 24 * 60 
            AS INTEGER
        )
    ) as avg_processing_time_minutes
FROM videos
WHERE processed_at IS NOT NULL
GROUP BY status;

-- Find videos pending processing (stuck jobs)
SELECT 
    v.id,
    v.uploaded_at,
    CAST(
        (julianday('now') - julianday(v.uploaded_at)) * 24 * 60 
        AS INTEGER
    ) as minutes_waiting
FROM videos v
WHERE v.status = 'processing'
  AND v.processed_at IS NULL
  AND julianday('now') - julianday(v.uploaded_at) > 0.0347  -- 50 minutes
ORDER BY v.uploaded_at ASC;
```

---

## API Endpoints

### 1. Upload Video

```http
POST /api/upload
Content-Type: multipart/form-data
Authorization: Bearer {token}

Request:
------WebKitFormBoundary
Content-Disposition: form-data; name="video"; filename="my-video.mp4"
Content-Type: video/mp4

[binary data]
------WebKitFormBoundary--

Response (Success):
HTTP/1.1 200 OK
Content-Type: application/json

{
  "video_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "processing",
  "uploaded_at": "2024-01-27T10:30:00Z"
}

Response (Error - File too large):
HTTP/1.1 413 Payload Too Large
{
  "error": "File size exceeds 2GB limit",
  "max_size": 2147483648
}

Response (Error - Invalid format):
HTTP/1.1 400 Bad Request
{
  "error": "Invalid video format",
  "supported_formats": ["mp4", "webm", "mov"]
}
```

### 2. Check Video Status

```http
GET /api/videos/{video_id}
Authorization: Bearer {token}

Response (Processing):
HTTP/1.1 200 OK
{
  "video_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "processing",
  "uploaded_at": "2024-01-27T10:30:00Z",
  "estimated_completion": "2024-01-27T10:31:30Z"
}

Response (Approved):
HTTP/1.1 200 OK
{
  "video_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "approved",
  "r2_url": "https://videos.example.com/550e8400...",
  "duration": 125,
  "uploaded_at": "2024-01-27T10:30:00Z",
  "processed_at": "2024-01-27T10:31:15Z"
}

Response (Duplicate):
HTTP/1.1 200 OK
{
  "video_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "duplicate",
  "duplicate_of": "660e8400-e29b-41d4-a716-446655440000",
  "similarity_score": 0.92,
  "original_video": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "uploaded_at": "2024-01-20T15:20:00Z",
    "uploader": "user123"
  }
}
```

### 3. List User Videos

```http
GET /api/videos?status=approved&limit=20&offset=0
Authorization: Bearer {token}

Response:
HTTP/1.1 200 OK
{
  "videos": [
    {
      "video_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "approved",
      "duration": 125,
      "uploaded_at": "2024-01-27T10:30:00Z",
      "thumbnail": "https://..."
    },
    ...
  ],
  "total": 45,
  "limit": 20,
  "offset": 0
}
```

### 4. Delete Video

```http
DELETE /api/videos/{video_id}
Authorization: Bearer {token}

Response:
HTTP/1.1 200 OK
{
  "message": "Video deleted successfully",
  "video_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### 5. Internal: Processor Callback (Not public)

```http
POST /internal/fingerprint-complete
Authorization: Internal-Secret {secret}

Request:
{
  "video_id": "550e8400-e29b-41d4-a716-446655440000",
  "video_vector": [0.2, 0.5, 0.8, ...],
  "audio_hash": "AQADtEmUaEkSRYmS...",
  "duration": 125,
  "frame_count": 62,
  "processing_time_ms": 8450
}

Response:
HTTP/1.1 200 OK
{
  "status": "received"
}
```

---

## Project Structure

```
video-platform/
│
├── workers/                          # Cloudflare Workers (Rust)
│   ├── upload-api/                   # Upload endpoint
│   │   ├── src/
│   │   │   ├── lib.rs               # Main worker logic
│   │   │   ├── validation.rs        # File validation
│   │   │   └── r2.rs                # R2 upload helpers
│   │   ├── Cargo.toml
│   │   └── wrangler.toml            # Cloudflare config
│   │
│   ├── queue-consumer/               # Fingerprint job processor
│   │   ├── src/
│   │   │   ├── lib.rs               # Queue handler
│   │   │   ├── vectorize.rs         # Vectorize queries
│   │   │   └── detector.rs          # Duplicate detection logic
│   │   ├── Cargo.toml
│   │   └── wrangler.toml
│   │
│   └── shared/                       # Shared types/utils
│       ├── src/
│       │   ├── lib.rs
│       │   ├── types.rs             # Common types
│       │   └── config.rs            # Configuration
│       └── Cargo.toml
│
├── processor/                        # External processor (Rust)
│   ├── src/
│   │   ├── main.rs                  # HTTP server
│   │   ├── fingerprint/
│   │   │   ├── mod.rs
│   │   │   ├── video.rs             # Video pHash
│   │   │   └── audio.rs             # Audio fingerprinting
│   │   ├── ffmpeg.rs                # FFmpeg wrapper
│   │   └── r2_client.rs             # Download from R2
│   ├── Cargo.toml
│   └── Dockerfile                   # For deployment
│
├── migrations/                       # D1 database migrations
│   ├── 0001_initial.sql
│   └── 0002_add_indexes.sql
│
├── scripts/                          # Utility scripts
│   ├── setup.sh                     # Initial setup
│   ├── deploy.sh                    # Deployment script
│   └── test-upload.sh               # Test upload endpoint
│
├── docs/                             # Documentation
│   ├── API.md
│   ├── DEPLOYMENT.md
│   └── ARCHITECTURE.md              # This file
│
├── .github/
│   └── workflows/
│       ├── deploy-workers.yml       # CI/CD for workers
│       └── deploy-processor.yml     # CI/CD for processor
│
├── Cargo.toml                        # Workspace config
└── README.md                         # Project README
```

---

## Setup & Installation

### Prerequisites

**1. Cloudflare Account**
- Sign up at https://dash.cloudflare.com
- Get your Account ID from dashboard
- Create API token with permissions:
  - Workers Scripts: Edit
  - Workers KV: Edit
  - Workers R2: Edit
  - D1: Edit
  - Vectorize: Edit

**2. Local Development Tools**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install wrangler (Cloudflare CLI)
npm install -g wrangler

# Install FFmpeg
# Ubuntu/Debian:
sudo apt update && sudo apt install -y ffmpeg libchromaprint-dev libchromaprint-tools

# macOS:
brew install ffmpeg chromaprint

# Verify installations
rustc --version    # Should be 1.70+
wrangler --version # Should be 3.0+
ffmpeg -version
fpcalc -version
```

### Step 1: Clone and Setup

```bash
# Clone repository
git clone https://github.com/yourusername/video-platform
cd video-platform

# Login to Cloudflare
wrangler login

# Initialize workspace
cargo build
```

### Step 2: Create Cloudflare Resources

```bash
# Create R2 bucket
wrangler r2 bucket create video-storage

# Create D1 database
wrangler d1 create video-db

# Create Queue
wrangler queues create video-processing

# Create Vectorize index
wrangler vectorize create video-fingerprints \
  --dimensions=64 \
  --metric=cosine

# Run migrations
wrangler d1 execute video-db --file=migrations/0001_initial.sql
```

### Step 3: Configure Workers

Edit `workers/upload-api/wrangler.toml`:

```toml
name = "video-upload-api"
main = "src/lib.rs"
compatibility_date = "2024-01-27"

[env.production]
workers_dev = false
route = { pattern = "api.yourdomain.com/*", zone_name = "yourdomain.com" }

[[r2_buckets]]
binding = "VIDEO_BUCKET"
bucket_name = "video-storage"

[[d1_databases]]
binding = "DB"
database_name = "video-db"
database_id = "your-database-id-here"

[[queues.producers]]
binding = "VIDEO_QUEUE"
queue = "video-processing"

[vars]
PROCESSOR_URL = "https://processor.yourdomain.com"
MAX_FILE_SIZE = "2147483648"  # 2GB
```

Edit `workers/queue-consumer/wrangler.toml`:

```toml
name = "video-queue-consumer"
main = "src/lib.rs"
compatibility_date = "2024-01-27"

[[queues.consumers]]
queue = "video-processing"
max_batch_size = 10
max_batch_timeout = 30
max_retries = 3

[[r2_buckets]]
binding = "VIDEO_BUCKET"
bucket_name = "video-storage"

[[d1_databases]]
binding = "DB"
database_name = "video-db"
database_id = "your-database-id-here"

[[vectorize]]
binding = "VECTORIZE"
index_name = "video-fingerprints"

[vars]
PROCESSOR_URL = "https://processor.yourdomain.com"
PROCESSOR_SECRET = "your-secret-key-here"
```

### Step 4: Deploy Workers

```bash
# Deploy upload API
cd workers/upload-api
wrangler deploy

# Deploy queue consumer
cd ../queue-consumer
wrangler deploy
```

### Step 5: Setup External Processor

```bash
cd processor

# Build processor
cargo build --release

# Run locally for testing
RUST_LOG=info cargo run

# Or build Docker image
docker build -t video-processor .

# Run with Docker
docker run -d \
  -p 8080:8080 \
  -e R2_ACCESS_KEY=your-key \
  -e R2_SECRET_KEY=your-secret \
  -e R2_ENDPOINT=https://your-account.r2.cloudflarestorage.com \
  video-processor

# Deploy to Railway/Fly.io/AWS
# See DEPLOYMENT.md for specific instructions
```

### Step 6: Test

```bash
# Test upload
curl -X POST https://api.yourdomain.com/upload \
  -H "Authorization: Bearer test-token" \
  -F "video=@test-video.mp4"

# Should return:
# {
#   "video_id": "550e8400-...",
#   "status": "processing"
# }

# Check status (wait 30-60 seconds)
curl https://api.yourdomain.com/videos/550e8400-... \
  -H "Authorization: Bearer test-token"
```

---

## Configuration

### Environment Variables

#### Upload Worker
```bash
PROCESSOR_URL=https://processor.yourdomain.com
MAX_FILE_SIZE=2147483648        # 2GB in bytes
ALLOWED_FORMATS=mp4,webm,mov
MAX_DURATION=3600               # 1 hour in seconds
```

#### Queue Consumer
```bash
PROCESSOR_URL=https://processor.yourdomain.com
PROCESSOR_SECRET=your-secret-key
SIMILARITY_THRESHOLD=0.85
MAX_RETRIES=3
```

#### Processor
```bash
R2_ACCESS_KEY=your-access-key
R2_SECRET_KEY=your-secret-key
R2_ENDPOINT=https://account-id.r2.cloudflarestorage.com
R2_BUCKET=video-storage
FRAME_INTERVAL=2                # Extract 1 frame per N seconds
PHASH_SIZE=8                    # 8x8 = 64 dimensions
PORT=8080
```

### Tuning Parameters

```rust
// workers/queue-consumer/src/lib.rs

// Similarity threshold (higher = stricter)
const DUPLICATE_THRESHOLD: f32 = 0.85;      // 85% similar = duplicate
const REVIEW_THRESHOLD: f32 = 0.70;         // 70-85% = needs review

// Vectorize query settings
const TOP_K: usize = 5;                     // Check top 5 matches
const MIN_SIMILARITY: f32 = 0.70;           // Ignore anything below 70%

// processor/src/fingerprint/video.rs

// Frame extraction
const FRAME_INTERVAL_SECS: u32 = 2;         // 1 frame per 2 seconds
const KEYFRAMES_ONLY: bool = true;          // Only extract I-frames

// pHash dimensions
const PHASH_SIZE: u32 = 8;                  // 8x8 = 64 dimensions
const RESIZE_SIZE: u32 = 32;                // Resize to 32x32 before DCT
```

---

## Deployment

### Deploying to Cloudflare Workers

```bash
# Production deployment
cd workers/upload-api
wrangler deploy --env production

cd ../queue-consumer
wrangler deploy --env production
```

### Deploying Processor

#### Option 1: Railway

```bash
# Install Railway CLI
npm install -g @railway/cli

# Login
railway login

# Create new project
railway init

# Add environment variables
railway variables set R2_ACCESS_KEY=...
railway variables set R2_SECRET_KEY=...

# Deploy
railway up
```

#### Option 2: Fly.io

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Login
fly auth login

# Create app
fly launch

# Set secrets
fly secrets set R2_ACCESS_KEY=...
fly secrets set R2_SECRET_KEY=...

# Deploy
fly deploy
```

#### Option 3: AWS EC2

```bash
# SSH to your instance
ssh ubuntu@your-instance-ip

# Install dependencies
sudo apt update
sudo apt install -y ffmpeg libchromaprint-dev build-essential

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yourusername/video-platform
cd video-platform/processor
cargo build --release

# Create systemd service
sudo nano /etc/systemd/system/video-processor.service
```

```ini
[Unit]
Description=Video Processor
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/video-platform/processor
Environment="R2_ACCESS_KEY=your-key"
Environment="R2_SECRET_KEY=your-secret"
Environment="R2_ENDPOINT=https://..."
ExecStart=/home/ubuntu/video-platform/processor/target/release/processor
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
# Start service
sudo systemctl daemon-reload
sudo systemctl enable video-processor
sudo systemctl start video-processor

# Check logs
sudo journalctl -u video-processor -f
```

---

## Performance & Scaling

### Current Limits

| Component | Limit | Can Scale To |
|-----------|-------|--------------|
| Upload Worker | 100 req/s | 10,000+ req/s |
| Queue Consumer | 100 concurrent | 1,000+ concurrent |
| Processor | 10 videos/min | 1,000+ videos/min |
| Vectorize | 1M vectors | 100M+ vectors |
| D1 | 100K rows | 10M+ rows |
| R2 Storage | Unlimited | Unlimited |

### Scaling Strategies

#### 1. Horizontal Scaling - Processor

```bash
# Run multiple processor instances behind load balancer

# Instance 1
docker run -p 8080:8080 video-processor

# Instance 2
docker run -p 8081:8080 video-processor

# Instance 3
docker run -p 8082:8080 video-processor

# Load balancer distributes requests
```

#### 2. Optimize Frame Extraction

```rust
// Instead of 1 frame per 2 seconds
const FRAME_INTERVAL_SECS: u32 = 5;    // 1 frame per 5 seconds

// Or extract only keyframes
const KEYFRAMES_ONLY: bool = true;
```

**Impact:**
- 60s video: 30 frames → 12 frames (60% faster)
- Accuracy: ~95% → ~92% (minimal drop)

#### 3. Batch Processing

```rust
// Process multiple videos in parallel
async fn process_batch(jobs: Vec<Job>) {
    let handles: Vec<_> = jobs
        .into_iter()
        .map(|job| tokio::spawn(process_video(job)))
        .collect();
    
    futures::future::join_all(handles).await;
}
```

#### 4. Caching

```rust
// Cache common fingerprints in Workers KV
if let Some(cached) = kv.get(&video_id).await? {
    return cached;
}

// Or use Redis for processor
let cached_fingerprint = redis.get(video_id)?;
```

### Performance Metrics

**Expected Processing Times:**

| Video Length | Frames | Processing Time | Total Time |
|--------------|--------|-----------------|------------|
| 30 seconds   | 15     | 5-10s          | 8-15s      |
| 1 minute     | 30     | 10-20s         | 15-25s     |
| 5 minutes    | 150    | 30-60s         | 40-70s     |
| 10 minutes   | 300    | 60-120s        | 75-135s    |

**Bottlenecks:**

1. **FFmpeg extraction**: 40% of processing time
2. **DCT computation**: 30% of processing time
3. **R2 download**: 20% of processing time
4. **Vectorize query**: <5% of processing time

---

## Cost Analysis

### Cloudflare Costs (Approximate)

**Workers:**
- Free tier: 100,000 requests/day
- Paid: $5/month for 10M requests

**R2 Storage:**
- Storage: $0.015/GB/month
- Class A operations (upload): $4.50/million
- Class B operations (download): $0.36/million
- Zero egress fees

**D1:**
- First 5M rows: Free
- Storage: $0.50/GB/month
- Reads: $0.001/1K rows
- Writes: $1.00/1M rows

**Queues:**
- Free tier: 1M operations/month
- Paid: $0.40/million operations

**Vectorize:**
- Free tier: 30M queries/month
- Storage: $0.04/1M dimensions/month

### Monthly Cost Estimate

**Small Platform (1,000 videos/month):**
```
Workers: Free tier
R2: 1000 videos × 100MB × $0.015 = $1.50
D1: Free tier
Queues: Free tier
Vectorize: Free tier
Processor: $5-10 (Railway/Fly.io)
────────────────────────────────
Total: ~$10/month
```

**Medium Platform (100,000 videos/month):**
```
Workers: $5
R2: 100K × 100MB × $0.015 = $150
    + 100K uploads × $4.50/1M = $0.45
D1: $5
Queues: $5
Vectorize: $10
Processor: $50 (multiple instances)
────────────────────────────────
Total: ~$225/month
```

**Large Platform (1M videos/month):**
```
Workers: $20
R2: 1M × 100MB × $0.015 = $1,500
    + 1M uploads × $4.50/1M = $4.50
D1: $50
Queues: $40
Vectorize: $100
Processor: $500 (scaled infrastructure)
────────────────────────────────
Total: ~$2,200/month
```

### Cost Optimization Tips

1. **Delete duplicates from R2** - Save 10-30% on storage
2. **Use keyframes only** - Reduce processing time by 50%
3. **Implement CDN caching** - Reduce R2 reads
4. **Compress fingerprints** - Reduce Vectorize storage
5. **Batch operations** - Reduce queue costs

---

## Monitoring & Logging

### CloudFlare Analytics

**Workers Analytics:**
```javascript
// In worker code
ctx.waitUntil(
  fetch('https://analytics-endpoint.com/log', {
    method: 'POST',
    body: JSON.stringify({
      video_id,
      processing_time_ms,
      status: 'complete'
    })
  })
);
```

### Logs

**View worker logs:**
```bash
# Real-time logs
wrangler tail upload-api

# Filtered logs
wrangler tail queue-consumer --status error
```

**Processor logs:**
```bash
# Systemd
sudo journalctl -u video-processor -f

# Docker
docker logs -f video-processor

# Fly.io
fly logs
```

### Metrics to Track

**Upload API:**
- Requests per second
- Upload success rate
- Average upload time
- Error rate by type

**Queue Consumer:**
- Jobs processed per minute
- Average processing time
- Retry rate
- Duplicate detection rate

**Processor:**
- CPU usage
- Memory usage
- Frame extraction time
- Fingerprint generation time

**Database:**
- Query latency (p50, p95, p99)
- Storage usage
- Write throughput

### Sample Monitoring Dashboard

```
┌─────────────────────────────────────────────────────┐
│ Video Platform - Monitoring Dashboard              │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Uploads Today:           1,234   (↑ 15%)          │
│  Processing Queue:           45   (normal)          │
│  Duplicates Detected:        89   (7.2%)           │
│  Average Processing Time:  32.5s  (good)           │
│                                                     │
│  ┌─────────────────────────────────────────────┐  │
│  │ Upload Rate (req/min)                        │  │
│  │     ▁▂▃▅▆██▇▆▅▃▂▁                           │  │
│  │                                              │  │
│  └─────────────────────────────────────────────┘  │
│                                                     │
│  Status Distribution:                               │
│  ████████████████████ Approved (1,050)            │
│  ██████ Processing (95)                            │
│  ████ Duplicate (89)                               │
│                                                     │
│  Recent Errors:                                     │
│  - 14:32 - FFmpeg timeout (retry succeeded)        │
│  - 14:15 - R2 upload failed (retry succeeded)      │
│                                                     │
└─────────────────────────────────────────────────────┘
```

---

## Troubleshooting

### Common Issues

#### 1. "Video stuck in 'processing' status"

**Symptoms:**
- Video uploaded successfully
- Status never changes from "processing"

**Diagnosis:**
```bash
# Check queue consumer logs
wrangler tail queue-consumer

# Check if job is in queue
# (Use Cloudflare dashboard → Queues)

# Check processor logs
fly logs  # or docker logs
```

**Solutions:**
```bash
# Retry manually
curl -X POST https://worker.../internal/retry \
  -H "Authorization: Internal-Secret ..." \
  -d '{"video_id": "..."}'

# Or update database directly
wrangler d1 execute video-db \
  --command "UPDATE videos SET status = 'failed' WHERE id = '...'"
```

#### 2. "High duplicate false positive rate"

**Symptoms:**
- Unique videos marked as duplicates
- Similar but different videos flagged

**Solutions:**
```rust
// Increase threshold
const DUPLICATE_THRESHOLD: f32 = 0.90;  // Was 0.85

// Add audio check requirement
if best_match.score >= 0.85 && audio_similarity >= 0.85 {
    // Both must match
}

// Implement manual review queue
if best_match.score >= 0.80 && best_match.score < 0.90 {
    return Status::NeedsReview;
}
```

#### 3. "Processor running out of memory"

**Symptoms:**
- OOM errors in processor logs
- Processing fails for long videos

**Solutions:**
```rust
// Process video in chunks
const MAX_FRAMES: usize = 100;  // Limit frames

// Or stream instead of loading full video
ffmpeg.input(video_url)
    .video_frames(100)  // Take only first 100 frames
    .output(...)
```

#### 4. "Vectorize queries timing out"

**Symptoms:**
- Slow duplicate detection
- Timeout errors

**Solutions:**
```rust
// Reduce top_k
const TOP_K: usize = 3;  // Was 5

// Increase threshold (fewer results)
const MIN_SIMILARITY: f32 = 0.80;  // Was 0.70

// Add timeout
let query = tokio::time::timeout(
    Duration::from_secs(5),
    vectorize.query(...)
).await?;
```

### Debug Mode

Enable detailed logging:

```rust
// workers/queue-consumer/src/lib.rs

#[event(queue)]
async fn main(batch: MessageBatch<Job>, env: Env, _ctx: Context) -> Result<()> {
    console_log!("Processing batch of {} jobs", batch.len());
    
    for message in batch.messages()? {
        let start = Instant::now();
        
        console_log!("Job: {:?}", message.body());
        
        match process_video(message.body(), &env).await {
            Ok(_) => {
                console_log!("✓ Success in {:?}", start.elapsed());
                message.ack();
            }
            Err(e) => {
                console_error!("✗ Error: {}", e);
                message.retry();
            }
        }
    }
    Ok(())
}
```

---

## Learning Resources

### Recommended YouTube Videos

1. **Perceptual Hashing & Image Similarity**
   - "Perceptual Hashing" by Computerphile
   - "How Shazam Works" by Vox

2. **Video Processing**
   - "FFmpeg Complete Guide" by The Coding Train
   - "Video Processing with Rust" (search on YouTube)

3. **Vector Databases**
   - "What are Vector Databases?" by IBM Technology
   - "Cosine Similarity Explained" by StatQuest

4. **YouTube's Content ID**
   - "How YouTube's Content ID Works" by Tom Scott
   - "Copyright Detection on YouTube" by YouTube Creators

5. **System Design**
   - "Designing YouTube" by Gaurav Sen
   - "Video Streaming Architecture" by Hussein Nasser

### Documentation

- Cloudflare Workers: https://developers.cloudflare.com/workers/
- Cloudflare R2: https://developers.cloudflare.com/r2/
- Cloudflare Vectorize: https://developers.cloudflare.com/vectorize/
- FFmpeg: https://ffmpeg.org/documentation.html
- Chromaprint: https://acoustid.org/chromaprint

### Academic Papers

1. **Perceptual Hashing**
   - "Image Hashing with Perceptual Similarity" (2006)
   
2. **Audio Fingerprinting**
   - "A Highly Robust Audio Fingerprinting System" (Haitsma & Kalker, 2002)

3. **Video Copy Detection**
   - "Large-Scale Video Copy Detection using Deep Neural Networks" (Google, 2016)

---

## FAQ

**Q: How accurate is the duplicate detection?**
A: With properly tuned thresholds, expect:
- 95%+ accuracy for exact or near-exact copies
- 85%+ for modified videos (cropped, watermarked, re-encoded)
- May miss heavily edited videos or compilations

**Q: Can it detect partial copies (only part of video used)?**
A: Basic version: No. Advanced version would need temporal segmentation.

**Q: What about audio-only duplicates?**
A: Yes! Audio fingerprinting works independently. Great for podcast/music detection.

**Q: How much does it cost to run?**
A: See Cost Analysis section. ~$10/month for small platform, ~$225 for medium, ~$2,200 for large.

**Q: Can I use different cloud providers?**
A: Yes! Replace:
- R2 → AWS S3 or Google Cloud Storage
- Vectorize → Pinecone, Milvus, or Qdrant
- D1 → PostgreSQL or MySQL
- Workers → AWS Lambda or Google Cloud Functions

**Q: How do I handle videos that are mirrored/flipped?**
A: pHash has some tolerance, but horizontal flip significantly changes the hash. You could:
1. Generate two hashes (normal + flipped)
2. Use more advanced features (SIFT, SURF)
3. Train a neural network

**Q: Can this scale to YouTube size (500+ hours uploaded per minute)?**
A: Not with this basic architecture. YouTube uses:
- Distributed computing clusters
- Custom hardware (TPUs)
- Advanced ML models
- Content ID system took years to build

**Q: What's the minimum video length for reliable detection?**
A: 10+ seconds recommended. Shorter videos have fewer frames = less reliable fingerprints.

---

## Conclusion

This architecture provides a solid foundation for video duplicate detection at scale. It balances:

- ✅ **Performance**: Sub-second queries, parallel processing
- ✅ **Cost**: Affordable for startups, scales with usage
- ✅ **Accuracy**: 90%+ duplicate detection rate
- ✅ **Scalability**: Handles thousands of videos/day
- ✅ **Reliability**: Queue-based processing, automatic retries

**Next Steps:**
1. Set up local development environment
2. Deploy to Cloudflare (free tier available)
3. Test with sample videos
4. Monitor and tune thresholds
5. Scale as needed

**Advanced Features to Add:**
- Thumbnail generation
- Video transcoding (different resolutions)
- Scene detection
- Object recognition
- ML-based similarity (ResNet, CLIP)
- Partial match detection
- Real-time processing pipeline

Good luck building your video platform! 🚀