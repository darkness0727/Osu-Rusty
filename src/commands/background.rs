use crate::{
    Context, Error,
    embeds::error::{FailedMapErr, failed_embed_custom, failed_map},
    utils::{
        discord_utils::{check_reply, check_reply_with_embed},
        osu_utils::{fetch_map, fetch_mapset, parse_beatmap_url},
    },
};
use poise::say_reply;

/// See an user's osu profile and stats
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = false,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    aliases("bg")
)]
pub async fn background(
    ctx: Context<'_>,
    #[description = "Specify a map#"] map: Option<String>,
) -> Result<(), Error> {
    let parse_result = parse_beatmap_url(
        &map.unwrap_or(
            ctx.data()
                .channel_map_db
                .get_channel_map(ctx.channel_id())
                .await
                .unwrap_or_default(),
        ),
    );

    if parse_result.map_id.is_none() && parse_result.mapset_id.is_none() {
        let embed = failed_embed_custom(String::from("Invalid Beatmap URL"));
        check_reply_with_embed(&ctx, embed).await;
        return Ok(());
    };

    if let Some(map_id) = parse_result.map_id {
        tracing::debug!("{map_id}");
        let Ok(Some(mapset)) = fetch_map(map_id).await.map(|m| m.mapset) else {
            check_reply_with_embed(&ctx, failed_map(FailedMapErr::MapNotFound)).await;
            return Ok(());
        };
        check_reply(say_reply(ctx, mapset.covers.card_2x).await);
        return Ok(());
    }

    if let Some(mapset_id) = parse_result.mapset_id {
        let Ok(mapset) = fetch_mapset(mapset_id).await else {
            check_reply_with_embed(&ctx, failed_map(FailedMapErr::MapNotFound)).await;
            return Ok(());
        };
        check_reply(say_reply(ctx, mapset.covers.card_2x).await);
        return Ok(());
    }

    check_reply_with_embed(&ctx, failed_map(FailedMapErr::FailedUrlParse)).await;
    Ok(())
}
