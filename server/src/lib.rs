use worker::*;

mod db;
mod routes;
mod types;
mod validation;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        // CORS preflight for all routes
        .options_async("/api/register", |_, _| async move { routes::handle_options() })
        .options_async("/api/scores", |_, _| async move { routes::handle_options() })
        .options_async("/api/leaderboard/:seed", |_, _| async move {
            routes::handle_options()
        })
        .options_async("/api/leaderboard/:seed/around/:player_id", |_, _| async move {
            routes::handle_options()
        })
        .options_async("/api/player/:player_id/stats", |_, _| async move {
            routes::handle_options()
        })
        .options_async("/api/player/:player_id", |_, _| async move {
            routes::handle_options()
        })
        // API routes
        .post_async("/api/register", |req, ctx| async move {
            routes::register(req, ctx.env).await
        })
        .post_async("/api/scores", |req, ctx| async move {
            routes::submit_score(req, ctx.env).await
        })
        .get_async("/api/leaderboard/:seed", |req, ctx| async move {
            let seed: u64 = ctx
                .param("seed")
                .ok_or_else(|| Error::RustError("Missing seed parameter".into()))?
                .parse()
                .map_err(|_| Error::RustError("Invalid seed parameter".into()))?;
            routes::get_leaderboard(req, ctx.env, seed as i64).await
        })
        .get_async(
            "/api/leaderboard/:seed/around/:player_id",
            |req, ctx| async move {
                let seed: u64 = ctx
                    .param("seed")
                    .ok_or_else(|| Error::RustError("Missing seed parameter".into()))?
                    .parse()
                    .map_err(|_| Error::RustError("Invalid seed parameter".into()))?;
                let player_id = ctx
                    .param("player_id")
                    .ok_or_else(|| Error::RustError("Missing player_id parameter".into()))?
                    .to_string();
                routes::get_leaderboard_around(req, ctx.env, seed as i64, player_id).await
            },
        )
        .get_async("/api/player/:player_id/stats", |_req, ctx| async move {
            let player_id = ctx
                .param("player_id")
                .ok_or_else(|| Error::RustError("Missing player_id parameter".into()))?
                .to_string();
            routes::get_player_stats(ctx.env, player_id).await
        })
        .patch_async("/api/player/:player_id", |req, ctx| async move {
            let player_id = ctx
                .param("player_id")
                .ok_or_else(|| Error::RustError("Missing player_id parameter".into()))?
                .to_string();
            routes::update_player(req, ctx.env, player_id).await
        })
        .run(req, env)
        .await
}
