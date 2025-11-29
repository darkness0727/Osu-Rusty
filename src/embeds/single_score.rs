use std::ops::Not;

use rosu_pp::Beatmap;
use rosu_v2::{
    model::Grade,
    prelude::{Score, UserExtended},
};
use serenity::all::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};

use crate::{
    embeds::FAIL_EMBED_COLOR,
    utils::{
        CommaFormat, CommaFormatFloat,
        osu_pp::{cal_failed_pp, cal_score_pp_perf, calculate_nc_stats, is_fc, map_stats},
        osu_utils::{
            BPM_EMOJI, TAIL_MISS_EMOJI, format_hits, format_slider_misses, formated_song_length,
            get_flag_url, grade_emoji, relative_timestamp, star_color_spectrum,
        },
    },
};

pub fn create(
    player: UserExtended,
    score: &Score,
    beatmap: Beatmap,
    best_index: Option<usize>,
) -> CreateEmbed {
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

    let song_length = formated_song_length(seconds_drain);
    let max_pp = map_stats.pp.format();
    let stars = map_stats.stars.two_decimal();
    let score_combo = score.max_combo;
    let map_combo = map_stats.combo;
    let pp = if score.grade == Grade::F {
        cal_failed_pp(score, score.mods.clone(), &beatmap)
            .unwrap_or_default()
            .two_decimal()
    } else {
        score
            .pp
            .unwrap_or_else(|| cal_score_pp_perf(perf_attrs.clone(), score) as f32)
            .two_decimal()
    };

    let stats = &score.statistics;

    let nc_stats = if is_fc(score, map_combo, map.count_sliders).not() {
        let nc_stats = calculate_nc_stats(perf_attrs, score, Some(&beatmap));

        let nc_pp = nc_stats.pp.format();
        let nc_acc = nc_stats.acc.two_decimal();
        let nc_formatted_hits =
            format_hits(nc_stats.n300, nc_stats.n100, nc_stats.n50, nc_stats.miss);

        let tail_miss = nc_stats.slider_tail_miss;

        let nc_tail_misses = (tail_miss > 0)
            .then(|| format!(" • {tail_miss}{TAIL_MISS_EMOJI}"))
            .unwrap_or_default();

        format!(
            "**If FC** (__{nc_pp} PP__)  • {nc_formatted_hits} • **{nc_acc}%**{nc_tail_misses}\n"
        )
    } else {
        Default::default()
    };

    let formatted_hits = format_hits(stats.great, stats.ok, stats.meh, stats.miss);
    let formatted_slider_stats = format_slider_misses(score, beatmap)
        .map(|s| format!(" •  {s}"))
        .unwrap_or_default();

    let failed_percent = if !score.passed {
        let percentage =
            (score.total_hits() as f32 / score.maximum_statistics.great as f32 * 100.0).round();
        format!("@{percentage}%",)
    } else {
        String::from("")
    };

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
        "{}{}\t{}%\t{}\t{}\t{}",
        grade_emoji(score.grade),
        failed_percent,
        score.accuracy.two_decimal(),
        formatted_mods,
        score.score.format(),
        time_ago,
    );

    let embed_field_value = format!(
        "{}\n{}",
        format!(
            "**{pp}**/{max_pp} PP • {formatted_hits} • **{score_combo}**/{map_combo}x {formatted_slider_stats}"
        ),
        format!(
            "{nc_stats}`CS: {cs} AR: {ar} OD: {od} HP: {hp}` • `{song_length}` • {BPM_EMOJI} **{bpm}**"
        )
    );

    let embed_footer = CreateEmbedFooter::new("")
        .text(format!(
            "Mapset by {} | {:?}",
            mapset.creator_name, mapset.status
        ))
        .icon_url("https://files.catbox.moe/7kcm1a");

    let description = best_index
        .map(|i| format!("**__Personal Best #{i}__**"))
        .unwrap_or_default();

    CreateEmbed::new()
        .author(embed_author)
        .thumbnail(&mapset.covers.list)
        .title(embed_title)
        .description(description)
        .field(embed_field_name, embed_field_value, false)
        .url(&map.url)
        .footer(embed_footer)
        .color(star_color_spectrum(stars))
}

static PP_GAINED_TEXT: &str = "**[(?)](https://discord.com/channels/1297750821219467264/1297838959854096454/# \"the amount of raw profile PP gained from this play accounting for previous scores on the map, this does not include bonus PP and the value is only accurate if this is the most recent top play\")**";
static MISSING_TEXT: &str = "**[(?)](https://discord.com/channels/1297750821219467264/1297838959854096454/# \"the top200 did not include this score likely because the api wasn't done processing but presumably the score is in there\")**";

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
