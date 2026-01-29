-- Migration number: 0002 	 2024-01-29T12:00:00Z

CREATE TABLE audio_hashes (
    video_id TEXT NOT NULL,
    hash INTEGER NOT NULL,
    time_offset INTEGER NOT NULL,
    FOREIGN KEY(video_id) REFERENCES videos(id)
);

CREATE INDEX idx_audio_hashes ON audio_hashes(hash);