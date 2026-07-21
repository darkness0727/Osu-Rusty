use crate::{
    Error,
    embeds::{
        error::{FailedMapErr, failed_embed, failed_embed_custom, failed_map, player_not_found_embed},
        score::{create},
    },
    utils::{
        discord_utils::{check_reply_with_embed, reply_with_embed}, osu_utils::{
            MAX_TOP_PLAY_COUNT, download_map_file, fetch_map, fetch_map_scores, fetch_mapset_from_diff, fetch_personal_bests, fetch_player, load_local_beatmap, parse_beatmap_url
        }
    },
};
use poise::Context as PoiseContext;

/// See an user's osu recent score with statistics
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = false,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    aliases("compare", "c")
)]
pub async fn score(
    ctx: PoiseContext<'_, (), Error>,
    #[description = "Specify a user"] name: String,
    #[description = "Specify a map difficulty"] map: String,
) -> Result<(), Error> {
    let parse_result = parse_beatmap_url(&map);

    let Some(map_id) = parse_result.map_id else {
        let embed = parse_result
            .mapset_id
            .is_some()
            .then(|| failed_map(FailedMapErr::ExpectedDifficulty))
            .unwrap_or_else(|| failed_map(FailedMapErr::FailedUrlParse));

        check_reply_with_embed(&ctx, embed).await;

        return Ok(());
    };

    let player_handle = tokio::spawn(fetch_player(name.clone()));
    let scores_handle = tokio::spawn(fetch_map_scores(name.clone(), map_id));
    let map_handle = tokio::spawn(fetch_map(map_id));
    let mapset_handle = tokio::spawn(fetch_mapset_from_diff(map_id));
    let top_plays_handle = tokio::spawn(fetch_personal_bests(name.clone(), MAX_TOP_PLAY_COUNT, 0));

    let player = match player_handle.await {
        Ok(Ok(player)) => player,
        _ => {
            let embed = player_not_found_embed(name);
            check_reply_with_embed(&ctx, embed).await;
            return Ok(());
        }
    };

    let (Ok(Ok(map)), Ok(Ok(mapset))) = (map_handle.await, mapset_handle.await) else {
        let err = String::from("Failed to fetch beatmap");
        check_reply_with_embed(&ctx, failed_embed_custom(err)).await;
        return Ok(());
    };

    let scores = match scores_handle.await {
        Ok(Ok(scores)) => scores,
        _ => vec![],
    };

    if let Err(err) = download_map_file(map_id).await {
        println!("{err}");
        check_reply_with_embed(&ctx, failed_embed()).await;
        return Ok(());
    }

    let Ok(beatmap) = load_local_beatmap(map_id) else {
        println!("failed to parse or missing beatmap");
        check_reply_with_embed(&ctx, failed_embed()).await;
        return Ok(());
    };

    let top_plays = top_plays_handle.await.ok().and_then(|t| t.ok()); 

    let embed = create(player, scores, beatmap, map, mapset, top_plays); 

    reply_with_embed(&ctx, embed.clone()).await?;

    Ok(())
}
