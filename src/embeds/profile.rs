use rosu_v2::prelude::UserExtended;
use serenity::all::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};

use crate::{
    embeds::{DEFAULT_EMBED_COLOR, error::failed_embed_custom},
    utils::{
        CommaFormat, CommaFormatFloat, 
    },
    osu_utils::{format_join_date, get_flag_url, playtime_in_hours,
        relative_timestamp,}
};

pub fn create(player: UserExtended) -> CreateEmbed {
    let player_name = player.username.to_string();
    let Some(stats) = player.statistics else {
        return failed_embed_custom(format!("`{player_name}` doesnt have enough info to show"));
    };

    let pp = stats.pp.format();
    let acc = stats.accuracy.format_acc();
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

    let embed_author = CreateEmbedAuthor::new("")
        .name(format!(
            "{player_name}: {pp}pp (#{global_rank} {country_code}{country_rank})"
        ))
        .url(format!(
            "https://osu.ppy.sh/users/{}/osu",
            player_name.clone()
        ))
        .icon_url(get_flag_url(country_code.to_string(), 256));

    let description = format!(
        "{}\n{}\n{}",
        format!("Accuracy: `{acc}` • Level: `{level}`"),
        format!("Playtime: `{playtime}` • Playcount: `{playcount}`"),
        format!("Medals: `{medal_count}` • Team: {team_linked_name}{peak_rank_with_timestamp}"),
    );

    let embed_footer = CreateEmbedFooter::new("")
        .text(join_date)
        .icon_url("https://files.catbox.moe/7kcm1a");

    CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .author(embed_author)
        .thumbnail(player.avatar_url)
        .description(description)
        .footer(embed_footer)
}
