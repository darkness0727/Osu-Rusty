use rosu_v2::request::UserId;

use crate::{
    Context, Error,
    embeds::{
        error::{account_not_linked, player_not_found_embed},
        profile::create,
    },
    utils::{discord_utils::check_reply_with_embed, osu_utils::fetch_player},
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
    let db = &ctx.data().db;
    let Some(user_id) = name
        .map(UserId::from)
        .or_else(|| db.get_user_id(ctx.author().id.get()).ok().flatten())
    else {
        check_reply_with_embed(&ctx, account_not_linked()).await;
        return Ok(());
    };

    check_reply_with_embed(
        &ctx,
        fetch_player(user_id.clone())
            .await
            .map(create)
            .unwrap_or_else(|_| player_not_found_embed(user_id.to_string())),
    )
    .await;
    Ok(())
}
