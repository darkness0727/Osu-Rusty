use crate::{
    Context, Error,
    utils::command_helpers::{fetch_player_or_reply, resolve_user_id, show_typing},
    embeds::{
        error::account_not_linked,
        profile::create,
    },
    utils::discord_utils::check_reply_with_embed,
};

/// See an user's osu profile and stats
#[poise::command(
    prefix_command,
    slash_command,
    guild_only = false,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel",
    aliases("osu")
)]
pub async fn profile(
    ctx: Context<'_>,
    #[description = "Specify a user"] name: Option<String>,
) -> Result<(), Error> {
    show_typing(&ctx).await?;
    let Some(user_id) = resolve_user_id(&ctx, name).await else {
        check_reply_with_embed(&ctx, account_not_linked()).await;
        return Ok(());
    };

    let embed = match fetch_player_or_reply(&ctx, &user_id).await {
        Some(player) => create(player),
        None => return Ok(()),
    };

    check_reply_with_embed(&ctx, embed).await;
    Ok(())
}
