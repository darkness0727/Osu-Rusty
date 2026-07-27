use rosu_v2::prelude::UserExtended;
use serenity::all::CreateEmbed;

use crate::{
    embeds::{
        DEFAULT_EMBED_COLOR,
        common::{CATBOX_FOOTER_ICON, build_embed_author},
        error::failed_embed_custom,
    },
    utils::{
        CommaFormat, CommaFormatFloat,
        osu_utils::{format_join_date, playtime_in_hours, relative_timestamp},
    },
};

pub fn create(player: UserExtended) -> CreateEmbed {
    let player_name = player.username.to_string();
    let Some(stats) = player.statistics else {
        return failed_embed_custom(format!("`{player_name}` doesnt have enough info to show"));
    };

    let pp = stats.pp.format();
    let acc = stats.accuracy.two_decimal();
    let playcount = stats.playcount.format();
    let level = stats.level.current.format();
    let playtime = playtime_in_hours(stats.playtime);
    let country_code = player.country_code;
    let join_date = format_join_date(player.join_date);

    let global_rank = stats
        .global_rank
        .map(|f| f.format())
        .unwrap_or("0".to_string());

    let country_rank = stats
        .country_rank
        .map(|f| f.format())
        .unwrap_or("0".to_string());

    let medal_count = player
        .medals
        .map(|f| f.len().to_string())
        .unwrap_or("-".to_string());

    let peak_rank_with_timestamp = player
        .highest_rank
        .as_ref()
        .map(|f| {
            format!(
                "\nPeak rank: `{}` • {}",
                f.rank.format(),
                relative_timestamp(f.updated_at)
            )
        })
        .unwrap_or("".to_string());

    let team_linked_name = player
        .team
        .as_ref()
        .map(|f| format!("[{}](https://osu.ppy.sh/teams/{})", f.name, f.id))
        .unwrap_or(String::from("`none`"));

    let embed_author = build_embed_author(
        &player_name, &pp, &global_rank, country_code.as_ref(), &country_rank,
    );

    let description = format!(
        "Accuracy: `{acc}%` • Level: `{level}`\nPlaytime: `{playtime}` • Playcount: `{playcount}`\nMedals: `{medal_count}` • Team: {team_linked_name}{peak_rank_with_timestamp}"
    );

    let embed_footer = serenity::all::CreateEmbedFooter::new("")
        .text(join_date)
        .icon_url(CATBOX_FOOTER_ICON);

    CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .author(embed_author)
        .thumbnail(player.avatar_url)
        .description(description)
        .footer(embed_footer)
}
