use rosu_pp::Beatmap;
use rosu_v2::{prelude::UserExtended, request::UserId};

use crate::{
    Context,
    embeds::error::{failed_embed, player_not_found_embed},
    utils::{
        discord_utils::check_reply_with_embed,
        osu_utils::{download_map_file, load_local_beatmap},
    },
};

pub async fn resolve_user_id(ctx: &Context<'_>, name: Option<String>) -> Option<UserId> {
    let db = &ctx.data().db;
    name.map(UserId::from)
        .or_else(|| db.get_user_id(ctx.author().id.get()).ok().flatten())
}

pub async fn fetch_player_or_reply(ctx: &Context<'_>, user_id: &UserId) -> Option<UserExtended> {
    match tokio::spawn(crate::utils::osu_utils::fetch_player(user_id.clone())).await {
        Ok(Ok(player)) => Some(player),
        _ => {
            check_reply_with_embed(ctx, player_not_found_embed(user_id.to_string())).await;
            None
        }
    }
}

pub async fn fetch_beatmap_or_reply(ctx: &Context<'_>, map_id: u32) -> Option<Beatmap> {
    if let Err(err) = download_map_file(map_id).await {
        tracing::error!("{err}");
        check_reply_with_embed(ctx, failed_embed()).await;
        return None;
    }
    match load_local_beatmap(map_id) {
        Ok(beatmap) => Some(beatmap),
        Err(_) => {
            tracing::warn!("failed to parse or missing beatmap");
            check_reply_with_embed(ctx, failed_embed()).await;
            None
        }
    }
}
