use crate::{
    Error,
    utils::discord_utils::check_reply_with_embed,
    embeds::{error::player_not_found_embed, profile::create},
    utils::osu_utils::fetch_player,
};
use poise::Context as PoiseContext;

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
    ctx: PoiseContext<'_, (), Error>,
    #[description = "Specify a user"] name: String,
) -> Result<(), Error> {
    check_reply_with_embed(
        &ctx,
        fetch_player(name.clone())
            .await
            .map(create)
            .unwrap_or_else(|_| player_not_found_embed(name)),
    )
    .await;
    Ok(())
}
