use worker::*;


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

            // 3. Trigger Processor (Direct Call)
            // Note: In a real app, you might want to use a service binding or just await this.
            // For now, we print a log because we haven't built the processor URL handling yet.
            console_log!("Video uploaded! ID: {}", id);
            
            // Example of how you WOULD call the processor if it had a URL:
            // let client = reqwest::Client::new();
            // let _ = client.post("https://processor-url/process")
            //     .json(&serde_json::json!({ "video_id": id, "r2_key": key }))
            //     .send()
            //     .await;

            Response::ok(format!("Uploaded video: {}", id))
        })
        .run(req, env)
        .await
}
