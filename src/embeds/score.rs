use std::ops::Not;

use rosu_pp::{Beatmap, any::PerformanceAttributes};
use rosu_v2::{
    model::Grade,
    prelude::{BeatmapExtended, BeatmapsetExtended, Score, UserExtended},
};
use serenity::all::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};

use crate::{
    embeds::error::failed_embed_custom,
    utils::{
        CommaFormat, CommaFormatFloat,
        osu_pp::{
            cal_failed_pp, cal_score_perf, calculate_nc_stats, is_fc, map_stats, pb_index_id_match,
        },
        osu_utils::{
            BPM_EMOJI, MISS_EMOJI, TAIL_MISS_EMOJI, format_hits, format_slider_misses,
            format_slider_tick_misses, formated_song_length, get_flag_url, grade_emoji,
            highest_pp_score, relative_timestamp, star_color_spectrum,
        },
    },
};

pub fn create(
    player: UserExtended,
    scores: Vec<Score>,
    beatmap: Beatmap,
    map_extended: BeatmapExtended,
    mapset_extended: BeatmapsetExtended,
    top_plays: Option<Vec<Score>>,
) -> CreateEmbed {
    let player_name = player.username.to_string();

    let Some(player_stats) = player.statistics else {
        return failed_embed_custom(String::from("Failed to fetch player info"));
    };

    let Some((score, other_scores)) = highest_pp_score(scores) else {
        return failed_embed_custom(String::from("No were scores found"));
    };
    let score = &score;

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

    let (perf_attrs, map_stats) = map_stats(&beatmap, mods_owned, map_extended.seconds_drain);

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
            .unwrap_or_else(|| cal_score_perf(perf_attrs.clone(), score).pp() as f32)
            .two_decimal()
    };

    let stats = &score.statistics;

    let nc_stats = if is_fc(score, map_combo, map_extended.count_sliders).not() {
        let nc_stats = calculate_nc_stats(perf_attrs.clone(), score, Some(&beatmap));

        let nc_pp = nc_stats.pp.format();
        let nc_acc = nc_stats.acc.two_decimal();
        let nc_formatted_hits =
            format_hits(nc_stats.n300, nc_stats.n100, nc_stats.n50, nc_stats.miss);

        let tail_miss = nc_stats.slider_tail_miss;

        let nc_tail_misses = if tail_miss > 0 { format!(" • {tail_miss}{TAIL_MISS_EMOJI}") } else { Default::default() };

        format!(
            "**If FC** (__{nc_pp} PP__)  • {nc_formatted_hits} • **{nc_acc}%**{nc_tail_misses}\n"
        )
    } else {
        Default::default()
    };

    let formatted_hits = format_hits(stats.great, stats.ok, stats.meh, stats.miss);
    let formatted_slider_stats = format_slider_misses(score, &beatmap)
        .map(|s| format!(" •  {s}"))
        .unwrap_or_default();

    let failed_percent = if !score.passed {
        let percentage =
            (score.total_hits() as f32 / score.maximum_statistics.great as f32 * 100.0).round();
        format!("@{percentage}%",)
    } else {
        String::from("")
    };

    let other_scores_text = other_scores_text(
        other_scores,
        perf_attrs,
        &beatmap,
        map_extended.count_sliders,
    );

    let description = top_plays
        .and_then(|t| pb_index_id_match(t, score))
        .map(|i| format!("__**Personal Best #{i}**__"))
        .unwrap_or_default();

    let embed_author = CreateEmbedAuthor::new("")
        .name(format!(
            "{player_name}: {player_pp}pp (#{global_rank} {country_code}{country_rank})"
        ))
        .url(format!("https://osu.ppy.sh/users/{}/osu", player_name))
        .icon_url(get_flag_url(country_code.to_string(), 256));

    let embed_title = format!(
        "{} - {} [{}] [{}★]",
        mapset_extended.artist,
        mapset_extended.title,
        map_extended.version,
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
        "**{pp}**/{max_pp} PP • {formatted_hits} • **{score_combo}**/{map_combo}x {formatted_slider_stats}\n\
         {nc_stats}`CS: {cs} AR: {ar} OD: {od} HP: {hp}` • `{song_length}` • {BPM_EMOJI} **{bpm}**\n\n"
    );

    let mut fields = vec![(embed_field_name, embed_field_value, false)];

    other_scores_text
        .iter()
        .for_each(|text| fields.push((String::from(""), String::from(text), false)));

    let embed_footer = CreateEmbedFooter::new("")
        .text(format!(
            "Mapset by {} | {:?}",
            mapset_extended.creator_name, mapset_extended.status
        ))
        .icon_url("https://files.catbox.moe/7kcm1a");

    CreateEmbed::new()
        .author(embed_author)
        .thumbnail(&mapset_extended.covers.list)
        .title(embed_title)
        .description(description)
        .fields(fields)
        .url(&map_extended.url)
        .footer(embed_footer)
        .color(star_color_spectrum(stars))
}

fn other_scores_text(
    scores: Vec<Score>,
    perf_attrs: PerformanceAttributes,
    beatmap: &Beatmap,
    slider_count: u32,
) -> Vec<String> {
    if scores.is_empty() {
        return vec![String::from("")];
    }
    let mut texts = vec![String::from("__Other scores on the beatmap:__")];

    for score in scores.iter() {
        if texts.len() >= 25 {
            break;
        };

        let score_performance = cal_score_perf(perf_attrs.clone(), score);

        let stars = score_performance.stars().format();
        let misses = is_fc(score, score_performance.max_combo(), slider_count)
            .then(|| String::from("FC"))
            .unwrap_or(format!("{}{}", score.statistics.miss, MISS_EMOJI));

        let pp = score
            .pp
            .unwrap_or_else(|| score_performance.pp() as f32)
            .two_decimal();

        let grade = grade_emoji(score.grade);
        let mods = score.mods.clone();
        let acc = score.accuracy.two_decimal();
        let combo = score.max_combo;
        let timestamp = relative_timestamp(score.ended_at);
        let tick_miss = format_slider_tick_misses(score, beatmap).unwrap_or_default();

        let new_score = format!(
            "\n{grade} **+{mods}** [{stars}★] {pp}pp ({acc}%) {combo}x • {misses}{tick_miss} {timestamp}"
        );

        let last_is_full = texts
            .last()
            .map(|s| s.len() + new_score.len() > 1024)
            .unwrap_or(true);

        if last_is_full {
            texts.push(new_score);
        } else {
            texts.last_mut().unwrap().push_str(&new_score);
        }
    }

    texts
}
