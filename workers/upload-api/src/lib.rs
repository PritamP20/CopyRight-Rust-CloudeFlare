use worker::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct CompleteRequest {
    video_id: String,
    hashes: Vec<String>,
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .post_async("/upload", |mut req, ctx| async move {
            let bucket = ctx.env.bucket("VIDEO_BUCKET")?;
            let bytes = req.bytes().await?;
            
            if bytes.is_empty() {
                return Response::error("File is empty", 400);
            }

            let id = uuid::Uuid::new_v4().to_string();
            let key = format!("videos/{}.mp4", id);
            bucket.put(key.clone(), bytes).execute().await?;

            // 2. Save metadata to D1
            let db = ctx.env.d1("DB")?;
            let statement = db.prepare("INSERT INTO videos (id, r2_key, user_id, status, uploaded_at) VALUES (?, ?, ?, ?, ?)");
            let query = statement.bind(&[
                id.clone().into(),
                key.clone().into(),
                "web-user".into(), // Placeholder user
                "processing".into(),
                worker::Date::now().to_string().into()
            ])?;
            query.run().await?;

            console_log!("Video uploaded! ID: {}", id);
            Response::ok(format!("Uploaded video: {}", id))
        })
        .post_async("/internal/complete", |mut req, ctx| async move {
            let body: Result<CompleteRequest> = req.json().await;
            if let Err(e) = body {
                return Response::error(format!("Bad Request: {}", e), 400);
            }
            let body = body.unwrap();
            let db = ctx.env.d1("DB")?;

            // 1. Check for duplicates (Simplified Exact Match for now)
            // In a full production LSH system, we'd check bands.
            // Here we check if any substantial number of hashes match exactly.
            let mut duplicate_id: Option<String> = None;
            
            // Optimization: Check a sample of hashes or all in a single query if possible.
            // D1 binds limit is high enough for a few hundred hashes? Maybe.
            // Let's just check the *middle* hash and *start* hash as a quick heuristic for now? 
            // Or just loop top 5.
            for hash in body.hashes.iter().take(5) {
                let stmt = db.prepare("SELECT video_id FROM video_hashes WHERE hash_value = ? LIMIT 1");
                let query = stmt.bind(&[hash.clone().into()])?;
                let result = query.first::<String>(Some("video_id")).await;
                if let Ok(Some(vid)) = result {
                    if vid != body.video_id {
                         duplicate_id = Some(vid);
                         break;
                    }
                }
            }
            
            if let Some(orig_id) = duplicate_id {
                // MARK AS DUPLICATE
                db.prepare("UPDATE videos SET status = 'duplicate', original_video_id = ? WHERE id = ?")
                    .bind(&[orig_id.clone().into(), body.video_id.clone().into()])?
                    .run().await?;
                
                return Response::error(format!("Duplicate of {}", orig_id), 409);
            }

            // 2. Insert Hashes (Store)
            // Batch insert is better, but D1 batching via execute_batch?
            // Or multiple statements.
            let mut statements = Vec::new();
            
            // Mark active
            statements.push(db.prepare("UPDATE videos SET status = 'active' WHERE id = ?").bind(&[body.video_id.clone().into()])?);

            for (i, hash) in body.hashes.iter().enumerate() {
                // Insert Hash
                statements.push(
                    db.prepare("INSERT INTO video_hashes (video_id, frame_index, hash_value) VALUES (?, ?, ?)")
                      .bind(&[body.video_id.clone().into(), (i as i32).into(), hash.clone().into()])?
                );
                
                // Insert Bands (LSH)
                // Assuming Hex String: 16 chars. 4 bands of 4 chars.
                if hash.len() == 16 {
                    for b in 0..4 {
                        let start = b * 4;
                        let end = start + 4;
                        if let Ok(val) = u16::from_str_radix(&hash[start..end], 16) {
                             statements.push(
                                db.prepare("INSERT INTO video_lsh_bands (video_id, band_index, band_value) VALUES (?, ?, ?)")
                                  .bind(&[body.video_id.clone().into(), (b as i32).into(), (val as i32).into()])?
                             );
                        }
                    }
                }
            }
            
            db.batch(statements).await?;

            Response::ok("Video processed and indexed")
        })
        .run(req, env)
        .await
}
