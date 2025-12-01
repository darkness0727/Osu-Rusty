use serenity::all::CreateEmbed;

use crate::embeds::{DEFAULT_EMBED_COLOR, FAIL_EMBED_COLOR};

pub fn player_not_found_embed(name: String) -> CreateEmbed {
    CreateEmbed::new()
        .color(FAIL_EMBED_COLOR)
        .description(format!("User `{name}` was not found"))
}

pub fn not_enough_scores(name: String, length: usize, only_passes: bool) -> CreateEmbed {
    let description = if only_passes {
        match length {
            0 => format!("`{name}` has no recent passes"),
            1 => format!("`{name}` only has {length} recent pass"),
            _ => format!("`{name}` only has {length} recent passes"),
        }
    } else {
        match length {
            0 => format!("`{name}` has no recent scores"),
            1 => format!("`{name}` only has {length} recent score"),
            _ => format!("`{name}` only has {length} recent scores"),
        }
    };

    CreateEmbed::new()
        .color(FAIL_EMBED_COLOR)
        .description(description)
}

pub fn failed_map(fail_type: FailedMapErr) -> CreateEmbed {
    let err = match fail_type {
        FailedMapErr::FailedUrlParse => "Invalid beatmap URL",
        FailedMapErr::MapNotFound => "Map not found",
        FailedMapErr::SetNotFound => "Mapset not found",
        FailedMapErr::ExpectedDifficulty => "Expected a map difficulty, found mapset"
    }.to_string();
    
    failed_embed_custom(err)
}

pub fn no_scores_found() -> CreateEmbed {
    CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description("No scores found on map".to_string())
}

pub enum FailedMapErr {
    FailedUrlParse,
    MapNotFound,
    SetNotFound,
    ExpectedDifficulty,
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
