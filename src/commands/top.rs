use crate::{
    Context, Error,
    utils::command_helpers::{fetch_beatmap_or_reply, fetch_player_or_reply, resolve_user_id},
    embeds::{
        error::{account_not_linked, not_enough_scores},
        recent::create,
    },
    utils::{
        discord_utils::{check_reply_with_embed, reply_with_embed},
        osu_utils::fetch_personal_bests,
    },
};

/// See an user's osu recent score with statistics
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = false,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    aliases("t")
)]
pub async fn top(
    ctx: Context<'_>,
    #[description = "Specify which score"] index: usize,
    #[description = "Specify a user"] name: Option<String>,
) -> Result<(), Error> {
    let Some(user_id) = resolve_user_id(&ctx, name).await else {
        check_reply_with_embed(&ctx, account_not_linked()).await;
        return Ok(());
    };

    let top_plays_handle = tokio::spawn(fetch_personal_bests(user_id.clone(), index, 0));

    let Some(player) = fetch_player_or_reply(&ctx, &user_id).await else {
        return Ok(());
    };

    let Ok(Ok(top_plays)) = top_plays_handle.await else {
        return Ok(());
    };

    if index == 0 || top_plays.len() < index {
        let embed = not_enough_scores(player.username.to_string(), top_plays.len(), false);
        check_reply_with_embed(&ctx, embed).await;
        return Ok(());
    }

    let score = top_plays[index - 1].clone();
    let map_id = score.map_id;

    let Some(beatmap) = fetch_beatmap_or_reply(&ctx, map_id).await else {
        return Ok(());
    };

    let top_play_index = top_plays.len();
    let embed = create(player, &score, beatmap, top_play_index.into());

    reply_with_embed(&ctx, embed).await?;

    Ok(())
}
