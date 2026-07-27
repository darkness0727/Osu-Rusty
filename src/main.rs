use std::sync::OnceLock;

use ::serenity::all::ClientBuilder;
use poise::serenity_prelude as serenity;
use rosu_v2::Osu;
use serenity::prelude::*;

use crate::{
    commands::{
        background::background,
        link::{link, unlink},
        profile::profile,
        recent::recent,
        score::score,
        top::top,
    },
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

pub static OSU_CLIENT: OnceLock<Osu> = OnceLock::new();
static OSU_URL_RE: OnceLock<regex::Regex> = OnceLock::new();
use utils::database::{ChannelMapDb, UserDb};

// Custom user data available to all commands
pub struct Data {
    pub db: UserDb,
    pub channel_map_db: ChannelMapDb,
}

pub type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
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

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message: msg } => {
            // return early if message doesnt contain osu urls to save processing
            let has_osu_url = msg.content.to_lowercase().contains("osu.ppy.sh")
                || msg.embeds.iter().any(|embed| {
                    embed.url.as_deref().is_some_and(|u| u.contains("osu.ppy.sh"))
                        || embed
                            .description
                            .as_deref()
                            .is_some_and(|d| d.contains("osu.ppy.sh"))
                });

            if !has_osu_url {
                return Ok(());
            }

            let re = OSU_URL_RE.get_or_init(|| {
                regex::Regex::new(r"https?://osu\.ppy\.sh/[^\s]+").unwrap()
            });

            let map_url = re
                .find(&msg.content)
                .map(|m| m.as_str().to_string())
                .or_else(|| {
                    msg.embeds.iter().find_map(|embed| {
                        embed
                            .url
                            .as_deref()
                            .and_then(|url| re.find(url).map(|m| m.as_str().to_string()))
                            .or_else(|| {
                                embed.description.as_deref().and_then(|desc| {
                                    re.find(desc).map(|m| m.as_str().to_string())
                                })
                            })
                    })
                });

            let Some(map_url) = map_url else {
                return Ok(());
            };

            data.channel_map_db
                .set_channel_map(msg.channel_id, map_url);
        }
        _ => {}
    }
    Ok(())
}

async fn start_discord_bot() {
    // Login with a bot token from the environment
    _ = dotenvy::from_path("./.env");
    let discord_token = dotenvy::var("DISCORD_TOKEN").expect("Missing `DISCORD_TOKEN` env var");
    let db = UserDb::new("osu_bot.redb").expect("Failed to open database");

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let options = poise::FrameworkOptions {
        commands: vec![
            profile(),
            recent(),
            top(),
            background(),
            score(),
            link(),
            unlink(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(dotenvy::var("BOT_PREFIX").expect("Missing `BOT_PREFIX` env var")),
            ..Default::default()
        },
        pre_command: |ctx| {
            Box::pin(async move {
                // show typing for prefix commands
                if matches!(ctx, poise::Context::Prefix(_)) {
                    _ = ctx
                        .channel_id()
                        .broadcast_typing(ctx.serenity_context())
                        .await;
                }
            })
        },
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler(ctx, event, framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                tracing::info!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .unwrap();
                Ok(Data {
                    db,
                    channel_map_db: ChannelMapDb::new(),
                })
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
