use std::path::Path;

use crate::{
    Error, FAIL_EMBED_COLOR, OSU_CLIENT,
    embeds::recent_embed::create_recent_embed,
    utils::{
        failed_embed, player_not_found_embed, reply_with_embed, save_file, wrap_in_tilde,
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

    let Ok(Ok(player)) = player_handle.await else {
        let embed = player_not_found_embed(name);

        reply_with_embed(&ctx, embed).await;
        return Ok(());
    };

    let name = player.username.to_string();

    let Ok(Ok(recent_scores)) = scores_handle.await else {
        let embed = CreateEmbed::new()
            .color(FAIL_EMBED_COLOR)
            .description(format!(
                "{} has no recent scores",
                wrap_in_tilde(name.clone())
            ));

        reply_with_embed(&ctx, embed).await;
        return Ok(());
    };

    let length = recent_scores.len();

    if length < index {
        let embed = CreateEmbed::new()
            .color(FAIL_EMBED_COLOR)
            .description(format!(
                "{} {} recent {}",
                wrap_in_tilde(name.clone()),
                if length == 0 {
                    "has no".to_string()
                } else {
                    format!("only has {}", length)
                },
                if length == 1 { "score" } else { "scores" }
            ));

        reply_with_embed(&ctx, embed).await;
        return Ok(());
    }

    let score = recent_scores[index - 1].clone();

    let path = format!("./resources/{}.osu", score.map_id);

    let map_data_exists = Path::new(&path).exists();

    if !map_data_exists {
        let map_data_url = format!("https://osu.ppy.sh/osu/{}", score.map_id);
        let Ok(map_response) = reqwest::get(&map_data_url).await else {
            reply_with_embed(&ctx, failed_embed()).await;
            return Ok(());
        };

        let Ok(map_data) = map_response.bytes().await else {
            reply_with_embed(&ctx, failed_embed()).await;
            return Ok(());
        };

        if save_file(map_data, &path).is_err() {
            reply_with_embed(&ctx, failed_embed()).await;
            return Ok(());
        };
    }

    let embed = create_recent_embed(player, score, path);

    reply_with_embed(&ctx, embed).await;

    Ok(())
}
