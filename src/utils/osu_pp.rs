use std::{collections::HashMap, ops::Not};

use crate::{
    Error,
    utils::{
        CommaFormatFloat,
        osu_utils::{MAX_TOP_PLAY_COUNT, download_map_file, fetch_map_scores, load_local_beatmap},
    },
};
use rosu_pp::{
    Beatmap, Difficulty, GradualPerformance,
    any::{PerformanceAttributes, ScoreState},
    osu::{Osu as Osu_Pp, OsuScoreOrigin, OsuScoreState},
};
use rosu_v2::prelude::{GameMod, GameMods, Score}; 

pub fn is_classic(mods: &GameMods) -> bool {
    mods.iter().any(|m| matches!(m, &GameMod::ClassicOsu(_)))
}

/// Returns if the score is present in the top200 and its index. if the PP is high enough be present in the top200, the index.
pub async fn is_in_pb(top_plays: Vec<Score>, score: &Score) -> Result<IsPbResult, Error> {
    let ranked = score.pp.is_some();

    let mut score_id_to_index = HashMap::with_capacity(top_plays.len());
    let mut map_id_to_pp = HashMap::with_capacity(top_plays.len());
    for (index, score) in top_plays.iter().enumerate() {
        score_id_to_index.insert(score.id, index);
        map_id_to_pp.insert(score.map_id, score.pp.unwrap_or_default());
    }

    // check for score ID match
    if let Some(i) = score_id_to_index.get(&score.id) {
        return Ok(IsPbResult::InPB(*i + 1));
    }

    let score_pp = match score.pp {
        Some(pp) => pp,
        None => cal_pp_download_beatmap(score).await? as f32,
    };

    // return NotPB if there is a different score on the same map with a higher PP
    if map_id_to_pp
        .get(&score.map_id)
        .is_some_and(|pp| *pp >= score_pp)
    {
        return Ok(IsPbResult::NotPB);
    }

    // return what index the play would be if it's pp is high enough to be a top200 score
    match top_plays.binary_search_by(|s| score_pp.total_cmp(&s.pp.unwrap_or_default())) {
        Err(pos) if ranked && pos < MAX_TOP_PLAY_COUNT => Ok(IsPbResult::MissingPB(pos + 1)),
        Err(pos) if pos < MAX_TOP_PLAY_COUNT => Ok(IsPbResult::IfRanked(pos + 1)),
        _ => Ok(IsPbResult::NotPB),
    }
}

pub enum IsPbResult {
    InPB(usize),
    MissingPB(usize),
    IfRanked(usize),
    NotPB,
}
/// how much raw profile PP gained from a play accounting for previous scores
/// only accurate if the score is your most recent top play, as otherwise
/// the top plays have changed making the value inaccurate
pub async fn pp_gained_from_play(
    top_plays: Vec<Score>,
    score: &Score,
    username: String,
) -> Result<f32, Error> {
    let Some(score_pp) = score.pp else {
        return Err("score is not ranked".into());
    };

    let mut top_without_score: Vec<Score> = Vec::with_capacity(top_plays.len());
    let mut add_extra_pp = false;

    for top_s in top_plays.iter() {
        // remove the score
        if score.id == top_s.id
            || (score.map_id == top_s.map_id && score_pp > top_s.pp.unwrap_or_default())
        {
            add_extra_pp = top_plays.len() >= MAX_TOP_PLAY_COUNT;
            continue;
        }

        // no PP gained if equal or better score already exists
        if score.map_id == top_s.map_id {
            return Err("better score found in top".into());
        }

        top_without_score.push(top_s.clone());
    }

    let mut top_pps: Vec<f32> = top_without_score
        .iter()
        .map(|s| s.pp.unwrap_or_default())
        .collect();

    // to compensate for the removed score we add another copy of the lowest PP score
    // if the user has 200 scores (then likely they have more scores outside of top 200)
    if add_extra_pp {
        top_pps.sort_by(|a, b| b.total_cmp(a));
        top_pps.push(*top_pps.last().unwrap_or(&0.0));
    }
    let map_scores = fetch_map_scores(username, score.map_id).await?;

    // remove newer map scores than the score, newer score cant be higher in PP
    // because we return if a newer score with higher PP is found;
    let filtered_map_pps = map_scores
        .iter()
        .filter(|s| score.ended_at > s.ended_at && score.id != s.id)
        .map(|s| s.pp.unwrap_or_default());

    let prev_score_pp = filtered_map_pps.reduce(f32::max).unwrap_or_default();
    let pp_from_current_score = what_if_pp(top_pps.clone(), score_pp);
    let pp_from_prev_score = what_if_pp(top_pps, prev_score_pp);

    let pp_gained = (pp_from_current_score - pp_from_prev_score) as f32;

    Ok(pp_gained)
}

/// check how much raw profile PP you would get from a score
pub fn what_if_pp(mut top_pps: Vec<f32>, score_pp: f32) -> f64 {
    top_pps.sort_by(|a, b| b.total_cmp(a));
    top_pps.truncate(MAX_TOP_PLAY_COUNT);

    let weighted_total_before: f64 = top_pps
        .iter()
        .enumerate()
        .map(|(index, pp)| *pp as f64 * 0.95f64.powi(index as i32))
        .sum();

    top_pps.push(score_pp);
    top_pps.sort_by(|a, b| b.total_cmp(a));
    top_pps.truncate(MAX_TOP_PLAY_COUNT);

    let weighted_total_after: f64 = top_pps
        .iter()
        .enumerate()
        .map(|(index, pp)| *pp as f64 * 0.95f64.powi(index as i32))
        .sum();

    weighted_total_after - weighted_total_before
}

pub fn is_fc(score: &Score, map_combo: u32, slider_count: u32) -> bool {
    if !is_classic(&score.mods) {
        let stats = &score.statistics;
        return stats.miss + stats.small_tick_miss + stats.large_tick_miss == 0;
    }

    let fc_threshold = map_combo as f32 - 0.1 * slider_count as f32;
    score.max_combo as f32 >= fc_threshold
}

pub async fn cal_pp_download_beatmap(score: &Score) -> Result<f64, Error> {
    download_map_file(score.map_id).await?;
    let beatmap = &load_local_beatmap(score.map_id)?;

    let stats = &score.statistics;
    let is_classic = is_classic(&score.mods);

    let mods = score.mods.clone();

    let diff_attrs = rosu_pp::Difficulty::new()
        .mods(mods.clone())
        .calculate_for_mode::<Osu_Pp>(beatmap)
        .unwrap();

    let score_pp = rosu_pp::Performance::new(diff_attrs)
        .mods(mods)
        .calculate()
        .performance()
        .mods(score.mods.clone())
        .combo(score.max_combo)
        .accuracy(score.accuracy as f64)
        .large_tick_hits(stats.large_tick_hit)
        .small_tick_hits(stats.small_tick_hit)
        .misses(stats.miss)
        .n50(stats.meh)
        .n100(stats.ok)
        .n300(stats.great)
        .lazer(!is_classic)
        .calculate()
        .pp();
    Ok(score_pp)
}

pub fn cal_score_pp_perf(perf_attrs: PerformanceAttributes, score: &Score) -> f64 {
    let stats = &score.statistics;
    let is_classic = is_classic(&score.mods);
    perf_attrs
        .performance()
        .mods(score.mods.clone())
        .combo(score.max_combo)
        .accuracy(score.accuracy as f64)
        .large_tick_hits(stats.large_tick_hit)
        .small_tick_hits(stats.small_tick_hit)
        .misses(stats.miss)
        .n50(stats.meh)
        .n100(stats.ok)
        .n300(stats.great)
        .lazer(!is_classic)
        .calculate()
        .pp()
}

pub fn cal_failed_pp(score: &Score, mods: GameMods, beatmap: &Beatmap) -> Option<f64> {
    let stats = &score.statistics;
    let difficulty = Difficulty::new().mods(mods.clone());
    let mut gradual = GradualPerformance::new(difficulty, beatmap);

    let mut state = ScoreState {
        n300: stats.great,
        n100: stats.ok,
        n50: stats.meh,
        misses: stats.miss,
        slider_end_hits: stats.slider_tail_hit,
        osu_large_tick_hits: stats.large_tick_hit,
        osu_small_tick_hits: stats.small_tick_hit,
        max_combo: score.max_combo,
        ..ScoreState::new()
    };

    if is_classic(&mods).not() {
        state.slider_end_hits = stats.slider_tail_hit;
        state.osu_small_tick_hits = stats.small_tick_hit;
        state.osu_large_tick_hits = stats.large_tick_hit;
    }

    let objects_hit = score.total_hits() as usize;

    let pp = gradual.nth(state, objects_hit - 1)?.pp();

    Some(pp)
}

pub fn map_stats(
    beatmap: &Beatmap,
    mods: GameMods,
    seconds_drain: u32,
) -> (PerformanceAttributes, MapStats) {
    let diff_attrs = rosu_pp::Difficulty::new()
        .mods(mods.clone())
        .calculate_for_mode::<Osu_Pp>(beatmap)
        .unwrap();

    let stars = diff_attrs.stars;
    let combo = diff_attrs.max_combo;

    let perf_attrs = rosu_pp::Performance::new(diff_attrs)
        .mods(mods.clone())
        .calculate();

    let pp = perf_attrs.pp();

    let stats = beatmap.attributes().mods(mods).build();

    let (ar, od, cs, hp) = (
        stats.ar.two_decimal(),
        stats.od.two_decimal(),
        stats.cs.two_decimal(),
        stats.hp.two_decimal(),
    );

    let bpm = (beatmap.bpm() * { stats.clock_rate }).two_decimal();
    let seconds_drain = (seconds_drain as f64 / stats.clock_rate) as u32;

    (
        perf_attrs,
        MapStats {
            ar,
            od,
            cs,
            hp,
            bpm,
            seconds_drain,
            stars,
            combo,
            pp,
        },
    )
}

pub fn calculate_nc_stats(perf_attrs: PerformanceAttributes, score: &Score) -> NoChokeStats {
    let stats = &score.statistics;
    let max_stats = &score.maximum_statistics;
    let is_classic = is_classic(&score.mods);

    let total_hits = score.total_hits();
    let ratio_300 = stats.great as f32 / total_hits as f32;
    let ratio_100 = stats.ok as f32 / total_hits as f32;

    let is_fail = !score.passed;

    let misses = if is_fail { max_stats.great - total_hits } else { stats.miss };
    let tail_hits = if is_fail { max_stats.slider_tail_hit } else { stats.slider_tail_hit };

    let miss_to_300 = (misses as f32 * ratio_300).round() as u32;
    let miss_to_100 = (misses as f32 * ratio_100).round() as u32;
    let miss_to_50 = misses - (miss_to_300 + miss_to_100);

    let (nc_300, nc_100, nc_50) = (
        stats.great + miss_to_300,
        stats.ok + miss_to_100,
        stats.meh + miss_to_50,
    );

    let state = OsuScoreState {
        n300: nc_300,
        n100: nc_100,
        n50: nc_50,
        misses: 0,
        slider_end_hits: tail_hits,
        large_tick_hits: max_stats.large_tick_hit,
        small_tick_hits: max_stats.small_tick_hit,
        ..OsuScoreState::new()
    };

    let nc_acc = if is_classic {
        state.accuracy(OsuScoreOrigin::Stable) * 100.0
    } else {
        state.accuracy(OsuScoreOrigin::WithSliderAcc {
            max_large_ticks: max_stats.large_tick_hit,
            max_slider_ends: tail_hits,
        }) * 100.0
    };

    let nc_attrs = perf_attrs
        .performance()
        .mods(score.mods.clone())
        .n300(nc_300)
        .n100(nc_100)
        .n50(nc_50)
        .slider_end_hits(tail_hits
        )
        .lazer(!is_classic)
        .calculate();

    let nc_pp = nc_attrs.pp();

    let slider_tail_miss = if is_classic.not() && is_fail.not() {
        max_stats.slider_tail_hit - stats.slider_tail_hit
    } else {
        0
    };

    NoChokeStats {
        n300: nc_300,
        n100: nc_100,
        n50: nc_50,
        miss: 0,
        slider_tail_miss,
        acc: nc_acc,
        pp: nc_pp,
    }
}

pub struct NoChokeStats {
    pub n300: u32,
    pub n100: u32,
    pub n50: u32,
    pub miss: u32,
    pub slider_tail_miss: u32,
    pub acc: f64,
    pub pp: f64,
}

pub struct MapStats {
    pub ar: f32,
    pub od: f32,
    pub cs: f32,
    pub hp: f32,
    pub bpm: f32,
    pub seconds_drain: u32,
    pub stars: f64,
    pub combo: u32,
    pub pp: f64,
}
