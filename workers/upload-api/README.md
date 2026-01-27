# Upload API Worker Setup

This worker handles video uploads. It receives a file, uploads it to R2, saves metadata to the D1 database, and queues a job for processing.

## 1. Setup Configuration (`wrangler.toml`)

You need to link your specific Cloudflare resources to this worker.

### Step 1: Get your Database ID
Run this command in your terminal: 
```bash
npx wrangler d1 list
```
Copy the `ID` for the `video-db` database.

### Step 2: Update `wrangler.toml`
Open `wrangler.toml` in this directory and replace `REPLACE_WITH_YOUR_DB_ID` with the ID you just copied.

```toml
[[d1_databases]]
binding = "DB"
database_name = "video-db"
database_id = "paste-your-id-here" 
```

## 2. Update Code (`src/lib.rs`)

The current `src/lib.rs` is incomplete. You need to update it to:
1.  Connect to the Database ("DB").
2.  Connect to the Queue ("VIDEO_QUEUE").
3.  Save the video metadata.

(Code snippets for this are provided in the main documentation or by your AI assistant).

## 3. Deploy

Once configured and coded:
```bash
npx wrangler deploy
```
