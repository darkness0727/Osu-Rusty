use serenity::builder::CreateEmbed;
use rosu_v2::request::UserId;

use crate::{
    Context, Error,
    embeds::{DEFAULT_EMBED_COLOR, error::player_not_found_embed},
    utils::{discord_utils::check_reply_with_embed, osu_utils::fetch_player},
};

/// Link your Osu! account to the bot 
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = false,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]

pub async fn link(
    ctx: Context<'_>,
    #[description = "your Osu! username or id"] username: String,
) -> Result<(), Error> {
    // 1. Get the Discord ID of the person running the command
    let discord_id = ctx.author().id.get();

    // 2. Access the database from our custom Data struct
    let db = &ctx.data().db;

    // 4. Send a confirmation message back to the Discord channel
    let embed = match fetch_player(username.clone()).await {
        Ok(player) => {
            db.set_user_id(discord_id, UserId::Id(player.user_id))?;
            CreateEmbed::new()
            .color(DEFAULT_EMBED_COLOR)
            .description(format!(
                "Successfully linked your Discord account to **[{}](https://osu.ppy.sh/users/{})**!", player.username, player.user_id
            ))
        }
        _ => player_not_found_embed(username),
    };

    check_reply_with_embed(&ctx, embed).await;

    Ok(())
}

/// Unlink your Osu! account from the bot 
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = false,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]

pub async fn unlink(
    ctx: Context<'_>,
) -> Result<(), Error> {
    // 1. Get the Discord ID of the person running the command
    let discord_id = ctx.author().id.get();

    // 2. Access the database from our custom Data struct
    let db = &ctx.data().db;

    // 4. Send a confirmation message back to the Discord channel
    db.remove_user_id(discord_id)?;
    let embed = CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description("Osu! account successfully unlinked");

    check_reply_with_embed(&ctx, embed).await;

    Ok(())
}
