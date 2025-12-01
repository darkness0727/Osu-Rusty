use crate::{
    Error,
    embeds::error::{FailedMapErr, failed_embed_custom, failed_map},
    utils::{
        discord_utils::{check_reply, check_reply_with_embed},
        osu_utils::{fetch_map, fetch_mapset, parse_beatmap_url},
    },
};
use poise::{Context as PoiseContext, say_reply};

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
    ctx: PoiseContext<'_, (), Error>,
    #[description = "Specify a map#"] map: String,
) -> Result<(), Error> {
    let ids = parse_beatmap_url(&map);
    if ids.map_id.is_none() && ids.mapset_id.is_none() {
        let embed = failed_embed_custom(String::from("Invalid Beatmap URL"));
        check_reply_with_embed(&ctx, embed).await;
        return Ok(());
    };

    let url = if ids.map_id.is_some() {
        let map_id = ids.map_id.unwrap();
        println!("{map_id}");
        let Ok(Some(mapset)) = fetch_map(map_id).await.map(|m| m.mapset) else {
            let embed = failed_map(FailedMapErr::MapNotFound);
            check_reply_with_embed(&ctx, embed).await;
            return Ok(());
        };
        mapset.covers.card_2x
    } else if ids.mapset_id.is_some() {
        let mapset_id = ids.mapset_id.unwrap();
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
