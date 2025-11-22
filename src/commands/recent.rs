use crate::{
    Error,
    embeds::{
        error::{failed_embed, not_enough_scores, player_not_found_embed},
        recent::{create, edit_if_ranked_pb, edit_missing_pb, edit_pb},
    },
    utils::{
        discord_utils::{check_reply_with_embed, edit_message_embed, reply_with_embed},
        osu_utils::{
            IsPbResult, download_map_file, fetch_personal_bests, fetch_player,
            fetch_recent_scores, is_in_pb, load_local_beatmap,
        },
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
    aliases("rs", "r", "r")
)]
pub async fn recent(
    ctx: PoiseContext<'_, (), Error>,
    #[description = "Specify a user"] name: String,
    #[description = "Specify which score"] index: Option<usize>,
) -> Result<(), Error> {
    let index = index.unwrap_or(1);

    let player_handle = tokio::spawn(fetch_player(name.clone()));
    let scores_handle = tokio::spawn(fetch_recent_scores(name.clone(), index));
    let top_plays_handle = tokio::spawn(fetch_personal_bests(name.clone(), 100, 0));
    let top_plays_handle2 = tokio::spawn(fetch_personal_bests(name.clone(), 100, 100));

    let player = match player_handle.await {
        Ok(Ok(player)) => player,
        _ => {
            let embed = player_not_found_embed(name);
            check_reply_with_embed(&ctx, embed).await;
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
        check_reply_with_embed(&ctx, embed).await;
        return Ok(());
    }

    let score = recent_scores[index - 1].clone();
    let map_id = score.map_id;
    
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

    let embed = create(player, &score, beatmap);

    let msg_handle = reply_with_embed(&ctx, embed.clone()).await?;

    let Ok(Ok(mut top_plays)) = top_plays_handle.await else {
        return Ok(());
    };

    let Ok(Ok(top_plays_second)) = top_plays_handle2.await else {
        return Ok(());
    };

    top_plays.extend(top_plays_second);

    let Ok(is_top_result) = is_in_pb(top_plays, &score).await else {
        return Ok(());
    };

    let updated_embed = match is_top_result {
        IsPbResult::InPB(index) => edit_pb(embed, index, 0.0),
        IsPbResult::MissingPB(index) => edit_missing_pb(embed, index, 0.0),
        IsPbResult::IfRanked(index) => edit_if_ranked_pb(embed, index),
        IsPbResult::NotPB => return Ok(()),
    };

    edit_message_embed(ctx, msg_handle, updated_embed).await;

    Ok(())
}
