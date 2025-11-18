use crate::Error;
use poise::{Context as PoiseContext, CreateReply};
use serenity::{
    Result as SerenityResult,
    all::{CreateAllowedMentions, CreateEmbed},
};

pub fn check_reply(result: SerenityResult<poise::ReplyHandle<'_>>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

pub async fn reply_with_embed(ctx: &PoiseContext<'_, (), Error>, embed: CreateEmbed) {
    let embed_reply = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(CreateAllowedMentions::default().replied_user(false));

    check_reply(ctx.send(embed_reply).await);
}
