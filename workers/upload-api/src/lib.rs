use worker::*;
use sqlx_d1::D1Connection;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .post_async("/upload", |mut req, ctx| async move {
            let bucket = ctx.env.bucket("VIDEO_BUCKET")?;

            let bytes = req.bytes().await?;

            let id = uuid::Uuid::new_v4().to_string();
            let key = format!("videos/{}.mp4", id);

            bucket.put(key, bytes).execute().await?;



            let d1 = ctx.env.d1("DB")?;
            let d1_conn = D1Connection::new(d1);

            Response::ok(format!("Uploaded video: {}", id))
        })
        .run(req, env)
        .await
}
