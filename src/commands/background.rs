use crate::{
    Context, Error,
    embeds::error::{FailedMapErr, failed_embed_custom, failed_map},
    utils::{
        command_helpers::show_typing,
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
    #[description = "Specify a map#"] map: String,
) -> Result<(), Error> {
    show_typing(&ctx).await?;
    let ids = parse_beatmap_url(&map);
    if ids.map_id.is_none() && ids.mapset_id.is_none() {
        let embed = failed_embed_custom(String::from("Invalid Beatmap URL"));
        check_reply_with_embed(&ctx, embed).await;
        return Ok(());
    };

    let url = if let Some(map_id) = ids.map_id {
        tracing::debug!("{map_id}");
        let Ok(Some(mapset)) = fetch_map(map_id).await.map(|m| m.mapset) else {
            let embed = failed_map(FailedMapErr::MapNotFound);
            check_reply_with_embed(&ctx, embed).await;
            return Ok(());
        };
        mapset.covers.card_2x
    } else if let Some(mapset_id) = ids.mapset_id {
        let Ok(mapset) = fetch_mapset(mapset_id).await else {
            let embed = failed_map(FailedMapErr::MapNotFound);
            check_reply_with_embed(&ctx, embed).await;
            return Ok(());
        };
        mapset.covers.card_2x
    } else {
        let embed = failed_map(FailedMapErr::FailedUrlParse);
        check_reply_with_embed(&ctx, embed).await;
        return Ok(());
    };

    check_reply(say_reply(ctx, url).await);

    Ok(())
}
