use rosu_pp::Beatmap;
use rosu_v2::{
    prelude::{Score, UserExtended},
};
use serenity::all::CreateEmbed;

use crate::{
    embeds::{FAIL_EMBED_COLOR, MISSING_TEXT, PP_GAINED_TEXT, common::compute_score_embed_parts},
    utils::CommaFormatFloat,
};

pub fn create(
    player: UserExtended,
    score: &Score,
    beatmap: Beatmap,
    best_index: Option<usize>,
) -> CreateEmbed {
    let (Some(map), Some(mapset)) = (&score.map, &score.mapset) else {
        return CreateEmbed::new()
            .color(FAIL_EMBED_COLOR)
            .description("failed to fetch info");
    };

    let Some(parts) = compute_score_embed_parts(
        &player,
        score,
        &beatmap,
        map.seconds_drain,
        map.count_sliders,
        &mapset.artist,
        &mapset.title,
        &mapset.creator_name,
        &format!("{:?}", mapset.status),
        &map.version,
    ) else {
        return CreateEmbed::new()
            .color(FAIL_EMBED_COLOR)
            .description("Failed to fetch player info");
    };

    let description = best_index
        .map(|i| format!("**__Personal Best #{i}__**"))
        .unwrap_or_default();

    CreateEmbed::new()
        .author(parts.embed_author)
        .thumbnail(&mapset.covers.list)
        .title(parts.embed_title)
        .description(description)
        .field(parts.embed_field_name, parts.embed_field_value, false)
        .url(&map.url)
        .footer(parts.embed_footer)
        .color(parts.stars_color)
}

pub fn edit_pb_recent(embed: CreateEmbed, pb_index: usize, pp_gained: f32) -> CreateEmbed {
    let description = format!(
        "**__Personal Best #{pb_index}__**  • Gained: **{}pp** {PP_GAINED_TEXT}",
        pp_gained.two_decimal()
    );

    embed.description(description)
}

pub fn edit_missing_pb_recent(embed: CreateEmbed, pb_index: usize, pp_gained: f32) -> CreateEmbed {
    let description = format!(
        "**__Personal Best #{pb_index}__** {MISSING_TEXT}  • Gained: **{}pp** {PP_GAINED_TEXT}",
        pp_gained.two_decimal()
    );

    embed.description(description)
}

pub fn edit_if_ranked_pb(embed: CreateEmbed, pb_index: usize) -> CreateEmbed {
    let description = format!("__Personal Best #{pb_index}__ • **If Ranked**");

    embed.description(description)
}
