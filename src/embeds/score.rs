use rosu_pp::{Beatmap, any::PerformanceAttributes};
use rosu_v2::{
    prelude::{BeatmapExtended, BeatmapsetExtended, Score, UserExtended},
};
use serenity::all::CreateEmbed;

use crate::{
    embeds::{
        common::compute_score_embed_parts, error::{failed_embed_custom, no_scores_found},
    }, utils::{
        CommaFormatFloat,
        osu_pp::{cal_score_perf, is_fc, pb_index_id_match},
        osu_utils::{
            MISS_EMOJI, format_slider_tick_misses, grade_emoji,
            highest_pp_score, relative_timestamp,
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
    let Some((score, other_scores)) = highest_pp_score(scores) else {
        return no_scores_found();
    };
    let score = &score;

    let Some(parts) = compute_score_embed_parts(
        &player,
        score,
        &beatmap,
        map_extended.seconds_drain,
        map_extended.count_sliders,
        &mapset_extended.artist,
        &mapset_extended.title,
        &mapset_extended.creator_name,
        &format!("{:?}", mapset_extended.status),
        &map_extended.version,
    ) else {
        return failed_embed_custom(String::from("Failed to fetch player info"));
    };

    let other_scores_text = other_scores_text(
        other_scores,
        parts.perf_attrs,
        &beatmap,
        map_extended.count_sliders,
    );

    let description = top_plays
        .as_ref()
        .and_then(|t| pb_index_id_match(t, score))
        .map(|i| format!("__**Personal Best #{i}**__"))
        .unwrap_or_default();

    let mut fields = vec![(parts.embed_field_name, parts.embed_field_value, false)];

    other_scores_text
        .iter()
        .for_each(|text| fields.push((String::from(""), String::from(text), false)));

    CreateEmbed::new()
        .author(parts.embed_author)
        .thumbnail(&mapset_extended.covers.list)
        .title(parts.embed_title)
        .description(description)
        .fields(fields)
        .url(&map_extended.url)
        .footer(parts.embed_footer)
        .color(parts.stars_color)
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
