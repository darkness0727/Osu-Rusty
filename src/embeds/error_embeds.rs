use serenity::all::CreateEmbed;

use crate::embeds::FAIL_EMBED_COLOR;

pub fn player_not_found_embed(name: String) -> CreateEmbed {
    CreateEmbed::new()
        .color(FAIL_EMBED_COLOR)
        .description(format!("User `{name}` was not found"))
}

pub fn failed_embed() -> CreateEmbed {
    CreateEmbed::new()
        .color(FAIL_EMBED_COLOR)
        .description("Something went wrong".to_string())
}

pub fn failed_embed_custom(custom_err: String) -> CreateEmbed {
    CreateEmbed::new()
        .color(FAIL_EMBED_COLOR)
        .description(custom_err)
}