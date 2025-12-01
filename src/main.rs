use ::serenity::all::ClientBuilder;
use once_cell::sync::OnceCell;
use poise::serenity_prelude as serenity;
use rosu_v2::Osu;
use serenity::prelude::*;

use crate::{
    commands::{background::background, profile::profile, recent::recent, score::score, top::top},
    resource_handler::create_all_dir,
    utils::osu_utils::login,
};

mod commands;
mod embeds;
pub mod resource_handler;
pub mod utils;

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
//type Context<'a> = poise::Context<'a, Data, Error>;

pub static OSU_CLIENT: OnceCell<Osu> = OnceCell::new();

#[tokio::main]
async fn main() {
    _ = create_all_dir();
    osu_login().await;
    start_discord_bot().await;
}

async fn osu_login() {
    let osu_client_secret =
        dotenvy::var("OSU_CLIENT_SECRET").expect("Missing `OSU_CLIENT_SECRET` env var");
    let osu_client_id_string =
        dotenvy::var("OSU_CLIENT_ID").expect("Missing `OSU_CLIENT_ID` env var");

    let osu_client_id: u64 = osu_client_id_string
        .parse::<u64>()
        .expect("Invalid 'OSU_CLIENT_ID'");

    let osu = login(osu_client_id, osu_client_secret).await.unwrap();
    _ = OSU_CLIENT.set(osu);
}

async fn start_discord_bot() {
    // Login with a bot token from the environment
    _ = dotenvy::from_path("./.env");
    let discord_token = dotenvy::var("DISCORD_TOKEN").expect("Missing `DISCORD_TOKEN` env var");

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let options = poise::FrameworkOptions {
        commands: vec![profile(), recent(), top(), background(), score()],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("?".into()),
            ..Default::default()
        },
        // // The global error handler for all error cases that may occur
        // // on_error: |error| Box::pin(on_error(error)),
        // // This code is run before every command
        // pre_command: |ctx| {
        //     Box::pin(async move {
        //         println!("Executing command {}...", ctx.command().qualified_name);
        //     })
        // },
        // // This code is run after a command if it was successful (returned Ok)
        // post_command: |ctx| {
        //     Box::pin(async move {
        //         println!("Executed command {}!", ctx.command().qualified_name);
        //     })
        // },
        // // Every command invocation must pass this check to continue execution
        // command_check: Some(|ctx| {
        //     Box::pin(async move {
        //         if ctx.author().id == 123456789 {
        //             return Ok(false);
        //         }
        //         Ok(true)
        //     })
        // }),
        // // Enforce command checks even for owners (enforced by default)
        // // Set to true to bypass checks, which is useful for testing
        // skip_checks_for_owners: false,
        // event_handler: |_ctx, event, _framework, _data| {
        //     Box::pin(async move {
        //         println!(
        //             "Got an event in event handler: {:?}",
        //             event.snake_case_name()
        //         );
        //         Ok(())
        //     })
        // },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .unwrap();
                Ok(())
            })
        })
        .options(options)
        .build();

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let discord_client = ClientBuilder::new(discord_token, intents)
        .framework(framework)
        .await;
    discord_client.unwrap().start().await.unwrap();
}
