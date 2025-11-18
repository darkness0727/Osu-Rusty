use rosu_pp::Beatmap;
use rosu_v2::prelude::{Score, UserExtended};
use serenity::all::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};

use crate::{
    embeds::FAIL_EMBED_COLOR,
    osu_utils::{
        BPM_EMOJI, format_hits, formatted_song_length, get_flag_url, grade_emoji,
        relative_timestamp, star_color_spectrum,
    },
    osu_utils::{calculate_nc_stats, calculate_score_pp, map_stats},
    utils::{CommaFormat, CommaFormatFloat},
};

pub fn create(player: UserExtended, score: Score, beatmap: Beatmap) -> CreateEmbed {
    let player_name = player.username.to_string();

    let (Some(player_stats), Some(map), Some(mapset)) =
        (player.statistics, &score.map, &score.mapset)
    else {
        return CreateEmbed::new()
            .color(FAIL_EMBED_COLOR)
            .description("failed to fetch info");
    };

    let player_pp = player_stats.pp.format();
    let country_code = player.country_code;
    let global_rank = player_stats
        .global_rank
        .map(|f| f.format())
        .unwrap_or_else(|| "0".to_string());
    let country_rank = player_stats
        .country_rank
        .map(|f| f.format())
        .unwrap_or_else(|| "0".to_string());

    let mods_owned = score.mods.clone();
    let formatted_mods = format!("+{}", mods_owned);
    let time_ago = relative_timestamp(score.ended_at);

    let (perf_attrs, map_stats) = map_stats(&beatmap, mods_owned, map.seconds_drain);

    let (ar, od, cs, hp, bpm, seconds_drain) = (
        map_stats.ar,
        map_stats.od,
        map_stats.cs,
        map_stats.hp,
        map_stats.bpm,
        map_stats.seconds_drain,
    );

    let song_length = formatted_song_length(seconds_drain);
    let max_pp = map_stats.pp.format();
    let stars = map_stats.stars.two_decimal();
    let map_max_combo = map_stats.combo;
    let pp = score
        .pp
        .unwrap_or_else(|| calculate_score_pp(perf_attrs.clone(), &score) as f32)
        .two_decimal();

    let nc_stats = calculate_nc_stats(perf_attrs, &score);

    let nc_pp = nc_stats.pp.format();
    let nc_acc = nc_stats.acc.format_acc();
    let nc_formatted_hits = format_hits(nc_stats.n300, nc_stats.n100, nc_stats.n50, nc_stats.miss);

    let stats = &score.statistics;
    let formatted_hits = format_hits(stats.great, stats.ok, stats.meh, stats.miss);

    let nc_stats_formatted = format!(
        "**If FC** (__{} PP__)  • {} • **{}**\n",
        nc_pp, nc_formatted_hits, nc_acc
    );

    let embed_author = CreateEmbedAuthor::new("")
        .name(format!(
            "{player_name}: {player_pp}pp (#{global_rank} {country_code}{country_rank})"
        ))
        .url(format!("https://osu.ppy.sh/users/{}/osu", player_name))
        .icon_url(get_flag_url(country_code.to_string(), 256));

    let embed_title = format!(
        "{} - {} [{}] [{}★]",
        mapset.artist,
        mapset.title,
        map.version,
        stars.format()
    );

    let embed_field_name = format!(
        "{}\t{}\t{}\t{}\t{}",
        grade_emoji(score.grade.to_string()),
        score.accuracy.format_acc(),
        formatted_mods,
        score.score.format(),
        time_ago,
    );

    let embed_field_value = format!(
        "**{}**/{} PP • {} • **{}**/{}x\n{}`CS: {} AR: {} OD: {} HP: {}` • `{}` • {BPM_EMOJI} **{}**",
        pp,
        max_pp,
        formatted_hits,
        score.max_combo,
        map_max_combo,
        nc_stats_formatted,
        cs,
        ar,
        od,
        hp,
        song_length,
        bpm,
    );

    let embed_footer = CreateEmbedFooter::new("")
        .text(format!(
            "Mapset by {} | {:?}",
            mapset.creator_name, mapset.status
        ))
        .icon_url("https://files.catbox.moe/7kcm1a");

    CreateEmbed::new()
        .author(embed_author)
        .thumbnail(&mapset.covers.list)
        .title(embed_title)
        .field(embed_field_name, embed_field_value, false)
        .url(&map.url)
        .footer(embed_footer)
        .color(star_color_spectrum(stars))
}
