use crate::{
    Error, discord_utils::reply_with_embed, embeds::{error::player_not_found_embed, profile::create}, osu_utils::fetch_player
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
    let player_result = fetch_player(name.clone()).await;

    let embed = match player_result {
        Ok(player_data) => create(player_data),
        Err(_) => player_not_found_embed(name),
    };

    reply_with_embed(&ctx, embed).await;
    Ok(())
}
