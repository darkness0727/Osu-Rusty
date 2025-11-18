use crate::{
    Error, FAIL_EMBED_COLOR, OSU_CLIENT,
    embeds::recent_embed::create_recent_embed,
    utils::{
        failed_embed, get_beatmap_locally, player_not_found_embed, reply_with_embed, save_map_osu_file, wrap_in_tilde
    },
};
use poise::Context as PoiseContext;
use serenity::all::CreateEmbed;

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
    let Some(osu_client) = OSU_CLIENT.get() else {
        println!("Err: tried to access osu client before it was intialized");
        return Ok(());
    };

    let index = index.unwrap_or(1);

    let player_handle = tokio::spawn(osu_client.user(&name).into_future());
    let scores_handle = tokio::spawn(
        osu_client
            .user_scores(&name)
            .recent()
            .limit(index)
            .into_future(),
    );

    let make_fail_embed = |desc: String| CreateEmbed::new().color(FAIL_EMBED_COLOR).description(desc);

    let player = match player_handle.await {
        Ok(Ok(player)) => player,
        _ => {
            let embed = player_not_found_embed(name);
            reply_with_embed(&ctx, embed).await;
            return Ok(());
        }
    };

    let player_name = player.username.to_string();
    let recent_scores = match scores_handle.await {
        Ok(Ok(scores)) => scores,
        _ => {
            let embed = make_fail_embed(format!("{} has no recent scores", wrap_in_tilde(player_name.clone())));
            reply_with_embed(&ctx, embed).await;
            return Ok(());
        }
    };

    let length = recent_scores.len();

    if length < index {
        let have_text = if length == 0 { "has no".to_string() } else { format!("only has {}", length) };
        let score_text = if length == 1 { "score" } else { "scores" };

        let embed = make_fail_embed(format!(
            "{} {} recent {}",
            wrap_in_tilde(player_name.clone()),
            have_text,
            score_text
        ));

        reply_with_embed(&ctx, embed).await;
        return Ok(());
    }

    let score = recent_scores[index - 1].clone();
    let map_id = score.map_id;

    if let Err(err) = save_map_osu_file(map_id).await {
        println!("{err}");
        reply_with_embed(&ctx, failed_embed()).await;
        return Ok(());
    }

    let Ok(beatmap) = get_beatmap_locally(map_id) else {
        println!("failed to parse or missing beatmap");
        reply_with_embed(&ctx, failed_embed()).await;
        return Ok(());
    };

    if beatmap.check_suspicion().is_err() {
        println!("Didn't parse suspicious beatmap");
        reply_with_embed(&ctx, failed_embed()).await;
    }

    let embed = create_recent_embed(player, score, beatmap);

    reply_with_embed(&ctx, embed).await;

    Ok(())
}
