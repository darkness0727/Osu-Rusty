use crate::Context;
use poise::{CreateReply, ReplyHandle};
use serenity::{
    Result as SerenityResult,
    all::{CreateAllowedMentions, CreateEmbed},
};

pub fn check_reply(result: SerenityResult<poise::ReplyHandle<'_>>) {
    if let Err(why) = result {
        tracing::error!("Error sending message: {:?}", why);
    }

}
pub fn check_edit(result: Result<(), serenity::Error>) {
    if let Err(why) = result {
        tracing::error!("Error editing message: {:?}", why);
    }
}

pub async fn check_reply_with_embed(ctx: &Context<'_>, embed: CreateEmbed) {
    let embed_reply = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(CreateAllowedMentions::default().replied_user(false));

    check_reply(ctx.send(embed_reply).await);
}

pub async fn reply_with_embed<'a>(
    ctx: &'a Context<'_>,
    embed: CreateEmbed,
) -> Result<ReplyHandle<'a>, serenity::Error> {
    let embed_reply = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(CreateAllowedMentions::default().replied_user(false));

    ctx.send(embed_reply).await
}

pub async fn edit_message_embed(ctx: Context<'_>, handle: ReplyHandle<'_>, embed: CreateEmbed) {
    let embed_reply = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(CreateAllowedMentions::default().replied_user(false));

    check_edit(handle.edit(ctx, embed_reply).await);
}