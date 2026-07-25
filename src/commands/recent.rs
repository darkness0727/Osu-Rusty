use crate::{
    Context, Error, embeds::{
        error::{account_not_linked, failed_embed, not_enough_scores, player_not_found_embed}, recent::{create, edit_if_ranked_pb, edit_missing_pb_recent, edit_pb_recent},
    }, utils::{
        discord_utils::{check_reply_with_embed, edit_message_embed, reply_with_embed},
        osu_pp::{IsPbResult, is_in_pb, pp_gained_from_play},
        osu_utils::{
            MAX_TOP_PLAY_COUNT, download_map_file, fetch_map_scores, fetch_personal_bests, fetch_player, fetch_recent_scores, load_local_beatmap
        },
    },
};
use rosu_v2::{model::Grade, request::UserId};

/// See an user's osu recent score with statistics
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = false,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    aliases("rs", "r")
)]
pub async fn recent(
    ctx: Context<'_>,
    #[description = "Specify a user"] name: Option<String>,
    #[description = "Specify which score"] index: Option<usize>,
    #[description = "Should only contain passes"] pass: Option<bool>,
) -> Result<(), Error> {
    let db = &ctx.data().db;
    let Some(user_id) = name
        .map(UserId::from)
        .or_else(|| db.get_user_id(ctx.author().id.get()).ok().flatten())
    else {
        check_reply_with_embed(&ctx, account_not_linked()).await;
        return Ok(());
    };

    let index = index.unwrap_or(1);
    let only_passes = pass.unwrap_or_default();

    let player_handle = tokio::spawn(fetch_player(user_id.clone()));
    let scores_handle = tokio::spawn(fetch_recent_scores(
        user_id.clone(),
        index,
        !only_passes,
    ));
    let top_plays_handle = tokio::spawn(fetch_personal_bests(user_id.clone(), MAX_TOP_PLAY_COUNT, 0));
    let player = match player_handle.await {
        Ok(Ok(player)) => player,
        _ => {
            let embed = player_not_found_embed(user_id.to_string());
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
        let embed = not_enough_scores(name, length, only_passes);
        check_reply_with_embed(&ctx, embed).await;
        return Ok(());
    }

    let score = recent_scores[index - 1].clone();
    let map_id = score.map_id;

    if let Err(err) = download_map_file(map_id).await {
        tracing::error!("{err}");
        check_reply_with_embed(&ctx, failed_embed()).await;
        return Ok(());
    }

    let Ok(beatmap) = load_local_beatmap(map_id) else {
        tracing::warn!("failed to parse or missing beatmap");
        check_reply_with_embed(&ctx, failed_embed()).await;
        return Ok(());
    };

    let embed = create(player, &score, beatmap, None);

    let msg_handle = reply_with_embed(&ctx, embed.clone()).await?;

    if score.grade == Grade::F {
        return Ok(())
    }

    let map_scores_handle = tokio::spawn(fetch_map_scores(user_id.clone(), map_id));

    let Ok(Ok(top_plays)) = top_plays_handle.await else {
        return Ok(());
    };

    let Ok(is_top_result) = is_in_pb(top_plays.clone(), &score).await else {
        return Ok(());
    };

    let updated_embed = match is_top_result {
        IsPbResult::InPB(index) => {
            let map_scores = map_scores_handle.await.ok().and_then(|r| r.ok()).unwrap_or_default();
            edit_pb_recent(
                embed,
                index,
                pp_gained_from_play(top_plays, &score, map_scores)
                    .await
                    .unwrap_or_default(),
            )
        }
        IsPbResult::MissingPB(index) => {
            let map_scores = map_scores_handle.await.ok().and_then(|r| r.ok()).unwrap_or_default();
            edit_missing_pb_recent(
                embed,
                index,
                pp_gained_from_play(top_plays, &score, map_scores)
                    .await
                    .unwrap_or_default(),
            )
        }
        IsPbResult::IfRanked(index) => edit_if_ranked_pb(embed, index),
        IsPbResult::NotPB => return Ok(()),
    };

    edit_message_embed(ctx, msg_handle, updated_embed).await;

    Ok(())
}
