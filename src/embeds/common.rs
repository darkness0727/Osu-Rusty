use rosu_pp::{Beatmap, any::PerformanceAttributes};
use rosu_v2::{
    model::Grade,
    prelude::{Score, UserExtended},
};
use serenity::all::{CreateEmbedAuthor, CreateEmbedFooter};

use crate::utils::{
    CommaFormat, CommaFormatFloat,
    osu_pp::{cal_failed_pp, cal_score_perf, calculate_nc_stats, is_fc, map_stats},
    osu_utils::{
        BPM_EMOJI, TAIL_MISS_EMOJI, format_hits, format_slider_misses, formated_song_length,
        get_flag_url, grade_emoji, relative_timestamp, star_color_spectrum,
    },
};

pub static CATBOX_FOOTER_ICON: &str = "https://files.catbox.moe/7kcm1a";
pub static OSU_PROFILE_URL_BASE: &str = "https://osu.ppy.sh/users/";

pub struct ScoreEmbedParts {
    pub embed_author: CreateEmbedAuthor,
    pub embed_footer: CreateEmbedFooter,
    pub embed_title: String,
    pub embed_field_name: String,
    pub embed_field_value: String,
    pub stars_color: i32,
    pub perf_attrs: PerformanceAttributes,
}

pub fn compute_score_embed_parts(
    player: &UserExtended,
    score: &Score,
    beatmap: &Beatmap,
    seconds_drain: u32,
    count_sliders: u32,
    mapset_artist: &str,
    mapset_title: &str,
    mapset_creator: &str,
    mapset_status: &str,
    map_version: &str,
) -> Option<ScoreEmbedParts> {
    let player_name = player.username.to_string();
    let player_stats = player.statistics.as_ref()?;

    let player_pp = player_stats.pp.format();
    let country_code = player.country_code.clone();
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

    let (perf_attrs, ms) = map_stats(beatmap, mods_owned, seconds_drain);

    let (ar, od, cs, hp, bpm, song_secs) = (
        ms.ar, ms.od, ms.cs, ms.hp, ms.bpm, ms.seconds_drain,
    );

    let song_length = formated_song_length(song_secs);
    let max_pp = ms.pp.format();
    let stars = ms.stars.two_decimal();
    let score_combo = score.max_combo;
    let map_combo = ms.combo;
    let pp = if score.grade == Grade::F {
        cal_failed_pp(score, score.mods.clone(), beatmap)
            .unwrap_or_default()
            .two_decimal()
    } else {
        score
            .pp
            .unwrap_or_else(|| cal_score_perf(perf_attrs.clone(), score).pp() as f32)
            .two_decimal()
    };

    let stats = &score.statistics;

    let nc_stats = if !is_fc(score, map_combo, count_sliders) {
        let nc_stats = calculate_nc_stats(perf_attrs.clone(), score, Some(beatmap));

        let nc_pp = nc_stats.pp.format();
        let nc_acc = nc_stats.acc.two_decimal();
        let nc_formatted_hits =
            format_hits(nc_stats.n300, nc_stats.n100, nc_stats.n50, nc_stats.miss);

        let tail_miss = nc_stats.slider_tail_miss;

        let nc_tail_misses = if tail_miss > 0 {
            format!(" • {tail_miss}{TAIL_MISS_EMOJI}")
        } else {
            Default::default()
        };

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

    let embed_author = build_embed_author(&player_name, &player_pp, &global_rank, &country_code, &country_rank);

    let embed_title = format!(
        "{} - {} [{}] [{}★]",
        mapset_artist, mapset_title, map_version, stars.format()
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
        "**{pp}**/{max_pp} PP • {formatted_hits} • **{score_combo}**/{map_combo}x {formatted_slider_stats}\n\
         {nc_stats}`CS: {cs} AR: {ar} OD: {od} HP: {hp}` • `{song_length}` • {BPM_EMOJI} **{bpm}**"
    );

    let embed_footer = CreateEmbedFooter::new("")
        .text(format!("Mapset by {} | {}", mapset_creator, mapset_status))
        .icon_url(CATBOX_FOOTER_ICON);

    let stars_color = star_color_spectrum(stars);

    Some(ScoreEmbedParts {
        embed_author,
        embed_footer,
        embed_title,
        embed_field_name,
        embed_field_value,
        stars_color,
        perf_attrs,
    })
}

pub fn build_embed_author(
    player_name: &str,
    player_pp: &str,
    global_rank: &str,
    country_code: &str,
    country_rank: &str,
) -> CreateEmbedAuthor {
    CreateEmbedAuthor::new("")
        .name(format!(
            "{player_name}: {player_pp}pp (#{global_rank} {country_code}{country_rank})"
        ))
        .url(format!("{OSU_PROFILE_URL_BASE}{player_name}/osu"))
        .icon_url(get_flag_url(country_code.to_string(), 256))
}
