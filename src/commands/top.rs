use rosu_v2::request::UserId;

use crate::{
    Context, Error,
    embeds::{
        error::{account_not_linked, failed_embed, player_not_found_embed},
        recent::create,
    },
    utils::{
        discord_utils::{check_reply_with_embed, reply_with_embed},
        osu_utils::{download_map_file, fetch_personal_bests, fetch_player, load_local_beatmap},
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
    let db = &ctx.data().db;
    let Some(user_id) = name
        .map(UserId::from)
        .or_else(|| db.get_user_id(ctx.author().id.get()).ok().flatten())
    else {
        check_reply_with_embed(&ctx, account_not_linked()).await;
        return Ok(());
    };

    let player_handle = tokio::spawn(fetch_player(user_id.clone()));
    let top_plays_handle = tokio::spawn(fetch_personal_bests(user_id.clone(), index, 0));

    let player = match player_handle.await {
        Ok(Ok(player)) => player,
        _ => {
            let embed = player_not_found_embed(user_id.to_string());
            check_reply_with_embed(&ctx, embed).await;
            return Ok(());
        }
    };

    let Ok(Ok(top_plays)) = top_plays_handle.await else {
        return Ok(());
    };

    let score = top_plays[index - 1].clone();
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

    let top_play_index = top_plays.len();

    let embed = create(player, &score, beatmap, top_play_index.into());

    reply_with_embed(&ctx, embed.clone()).await?;

    Ok(())
}
