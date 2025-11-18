use crate::{Error, OSU_CLIENT, embeds::profile_embed::create_profile_embed, utils::{check_reply, player_not_found_embed}};
use poise::{Context as PoiseContext, CreateReply};
use serenity::all::CreateAllowedMentions;

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
    let Some(osu_client) = OSU_CLIENT.get() else {
        println!("Err: tried to access osu client before it was intialized");
        return Ok(());
    };

    let player = osu_client.user(&name).await;

    let embed = match player {
        Ok(player_data) => create_profile_embed(player_data),
        Err(_) => player_not_found_embed(name),
    };

    let embed_reply = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(CreateAllowedMentions::default().replied_user(false));

    check_reply(ctx.send(embed_reply).await);
    Ok(())
}
