use crate::{
    Error,
    discord_utils::reply_with_embed,
    embeds::{
        error::{failed_embed, not_enough_scores, player_not_found_embed},
        recent::create,
    },
    osu_utils::{download_map_file, fetch_player, fetch_scores, load_local_beatmap},
};
use poise::Context as PoiseContext;
/// See an user's osu recent score with statistics
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = false,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    aliases("rs", "r", "r")
)]
pub async fn recent(
    ctx: PoiseContext<'_, (), Error>,
    #[description = "Specify a user"] name: String,
    #[description = "Specify which score"] index: Option<usize>,
) -> Result<(), Error> {
    let index = index.unwrap_or(1);

    let player_handle = tokio::spawn(fetch_player(name.clone()));
    let scores_handle = tokio::spawn(fetch_scores(name.clone(), index));

    let player = match player_handle.await {
        Ok(Ok(player)) => player,
        _ => {
            let embed = player_not_found_embed(name);
            reply_with_embed(&ctx, embed).await;
            return Ok(());
        }
    };
    let name = player.username.to_string();

    let recent_scores = match scores_handle.await {
        Ok(Ok(scores)) => scores,
        _ => vec![],
    };

    let length = recent_scores.len();
    if length < index {
        let embed = not_enough_scores(name, length);
        reply_with_embed(&ctx, embed).await;
        return Ok(());
    }

    let score = recent_scores[index - 1].clone();
    let map_id = score.map_id;

    if let Err(err) = download_map_file(map_id).await {
        println!("{err}");
        reply_with_embed(&ctx, failed_embed()).await;
        return Ok(());
    }

    let Ok(beatmap) = load_local_beatmap(map_id) else {
        println!("failed to parse or missing beatmap");
        reply_with_embed(&ctx, failed_embed()).await;
        return Ok(());
    };

    let embed = create(player, score, beatmap);

    reply_with_embed(&ctx, embed).await;

    Ok(())
}
