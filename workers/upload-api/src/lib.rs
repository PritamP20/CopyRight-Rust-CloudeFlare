use worker::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct AudioHash {
    hash: u64,
    time_offset: u32,
}

#[derive(Deserialize, Serialize)]
struct CompleteRequest {
    video_id: String,
    hashes: Vec<String>,
    audio_hashes: Vec<AudioHash>,
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

            let db = ctx.env.d1("DB")?;
            let statement = db.prepare("INSERT INTO videos (id, r2_key, user_id, status, uploaded_at) VALUES (?, ?, ?, ?, ?)");
            let query = statement.bind(&[
                id.clone().into(),
                key.clone().into(),
                "web-user".into(), 
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

            let mut duplicate_id: Option<String> = None;
            
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
            
            if duplicate_id.is_none() {
                 for hash in body.audio_hashes.iter().take(20) {
                    let val = hash.hash as i64;
                    
                    let stmt = db.prepare("SELECT video_id FROM audio_hashes WHERE hash = ? LIMIT 1");
                    let query = stmt.bind(&[val.into()])?;
                    let result = query.first::<String>(Some("video_id")).await;
                    if let Ok(Some(vid)) = result {
                        if vid != body.video_id {
                             duplicate_id = Some(vid);
                             break;
                        }
                    }
                 }
            }
            
            if let Some(orig_id) = duplicate_id {
                db.prepare("UPDATE videos SET status = 'duplicate', original_video_id = ? WHERE id = ?")
                    .bind(&[orig_id.clone().into(), body.video_id.clone().into()])?
                    .run().await?;
                
                return Response::error(format!("Duplicate of {}", orig_id), 409);
            }

            let mut statements = Vec::new();
            
            statements.push(db.prepare("UPDATE videos SET status = 'active' WHERE id = ?").bind(&[body.video_id.clone().into()])?);

            for (i, hash) in body.hashes.iter().enumerate() {
                statements.push(
                    db.prepare("INSERT INTO video_hashes (video_id, frame_index, hash_value) VALUES (?, ?, ?)")
                      .bind(&[body.video_id.clone().into(), (i as i32).into(), hash.clone().into()])?
                );
                
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
            
            for hash in body.audio_hashes.iter() {
                statements.push(
                    db.prepare("INSERT INTO audio_hashes (video_id, hash, time_offset) VALUES (?, ?, ?)")
                      .bind(&[
                          body.video_id.clone().into(), 
                          (hash.hash as i64).into(), 
                          (hash.time_offset as i32).into()
                      ])?
                );
            }
            
            for chunk in statements.chunks(100) {
                 db.batch(chunk.to_vec()).await?;
            }

            Response::ok("Video processed and indexed")
        })
        .run(req, env)
        .await
}
