use std::fs;

use rosu_v2::prelude::{Score, UserExtended};
use serenity::all::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};

use crate::{
    FAIL_EMBED_COLOR,
    osu::osu_utils::{calculate_nc_stats, calculate_score_pp, map_max_stats, modded_map_stats},
    utils::{
        BPM_EMOJI, CommaFormat, CommaFormatFloat, discord_time_ago, failed_embed,
        failed_embed_custom, format_hits, formatted_song_length, get_flag_url, grade_emoji,
        star_color_spectrum,
    },
};

pub fn create_recent_embed(
    player: UserExtended,
    score: Score,
    map_data_path: String,
) -> CreateEmbed {
    let player_name = player.username.to_string();

    let (Some(player_stats), Some(map), Some(mapset)) =
        (player.statistics, &score.map, &score.mapset)
    else {
        return CreateEmbed::new()
            .color(FAIL_EMBED_COLOR)
            .description("failed to fetch info");
    };

    let Ok(beatmap) = rosu_pp::Beatmap::from_path(&map_data_path) else {
        _ = fs::remove_file(&map_data_path);
        return failed_embed();
    };

    if beatmap.check_suspicion().is_err() {
        return failed_embed_custom("Failed to parse beatmap info".to_string());
    }

    let player_pp = player_stats.pp.format();
    let country_code = player.country_code;

    let global_rank = player_stats
        .global_rank
        .map(|f| f.format())
        .unwrap_or("0".to_string());

    let country_rank = player_stats
        .country_rank
        .map(|f| f.format())
        .unwrap_or("0".to_string());

    let formatted_mods = format!("+{}", score.mods);
    let time_ago = discord_time_ago(score.ended_at);

    let map_stats = modded_map_stats(&beatmap, score.mods.clone().into(), map.seconds_drain);

    let (ar, od, cs, hp, bpm, seconds_drain) = (
        map_stats.ar,
        map_stats.od,
        map_stats.cs,
        map_stats.hp,
        map_stats.bpm,
        map_stats.seconds_drain,
    );

    let song_length = formatted_song_length(seconds_drain);

    let max_map_result = map_max_stats(&beatmap, score.mods.clone().into());

    let perf_attrs = max_map_result.perf_attrs;
    let max_pp = max_map_result.pp;
    let stars = max_map_result.stars;
    let map_max_combo = max_map_result.combo;

    let nc_stats = calculate_nc_stats(perf_attrs, &score);

    let perf_attrs = nc_stats.perf_attrs;
    let nc_pp = nc_stats.pp;
    let nc_acc = nc_stats.acc;
    let nc_formatted_hits = format_hits(nc_stats.n300, nc_stats.n100, nc_stats.n50, nc_stats.miss);

    let stats = &score.statistics;
    let formatted_hits = format_hits(stats.great, stats.ok, stats.meh, stats.miss);

    let pp = score
        .pp
        .unwrap_or_else(|| calculate_score_pp(perf_attrs, &score).1 as f32)
        .two_decimal();

    let nc_stats_formatted = format!(
        "**If FC** (__{} PP__)  • {} • **{}**\n",
        nc_pp, nc_formatted_hits, nc_acc
    );

    let embed_author = CreateEmbedAuthor::new("")
        .name(format!(
            "{player_name}: {player_pp}pp (#{global_rank} {country_code}{country_rank})"
        ))
        .url(format!(
            "https://osu.ppy.sh/users/{}/osu",
            player_name.clone()
        ))
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
        .color(star_color_spectrum(stars as f32))
}
