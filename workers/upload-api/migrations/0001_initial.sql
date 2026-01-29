-- Migration number: 0001 	 2024-01-29T00:00:00Z

DROP TABLE IF EXISTS video_lsh_bands;
DROP TABLE IF EXISTS video_hashes;
DROP TABLE IF EXISTS videos;

CREATE TABLE videos (
    id TEXT PRIMARY KEY,
    r2_key TEXT NOT NULL,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL,
    original_video_id TEXT, 
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    uploaded_at TEXT
);

CREATE TABLE video_hashes (
    video_id TEXT NOT NULL,
    frame_index INTEGER NOT NULL,
    hash_value TEXT NOT NULL, 
    FOREIGN KEY(video_id) REFERENCES videos(id)
);

CREATE TABLE video_lsh_bands (
    video_id TEXT NOT NULL,
    band_index INTEGER NOT NULL,
    band_value INTEGER NOT NULL, 
    FOREIGN KEY(video_id) REFERENCES videos(id)
);

CREATE INDEX idx_lsh_bands ON video_lsh_bands(band_index, band_value);
