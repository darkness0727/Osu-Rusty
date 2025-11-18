use crate::utils::{CommaFormatFloat, is_classic};
use rosu_pp::{
    Beatmap, GameMods,
    any::PerformanceAttributes,
    osu::{Osu, OsuScoreOrigin, OsuScoreState},
};
use rosu_v2::prelude::Score;

pub fn calculate_score_pp(perf_attrs: PerformanceAttributes, score: &Score) -> (PerformanceAttributes, f64) {
    let stats = &score.statistics;
    let is_classic = is_classic(&score.mods);
    let perf_attrs = perf_attrs
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
        .calculate();
    let pp = perf_attrs.pp();
    (perf_attrs, pp)
}

pub fn modded_map_stats(beatmap: &Beatmap, mods: GameMods, seconds_drain: u32) -> MapStats {
    let stats = beatmap.attributes().mods(mods).build();

    let (ar, od, cs, hp) = (
        stats.ar.two_decimal(),
        stats.od.two_decimal(),
        stats.cs.two_decimal(),
        stats.hp.two_decimal(),
    );

    let bpm = (beatmap.bpm() * { stats.clock_rate }).two_decimal();
    let drain = (seconds_drain as f64 / stats.clock_rate) as u32;

    MapStats {
        ar,
        od,
        cs,
        hp,
        bpm,
        seconds_drain: drain,
    }
}

pub fn map_max_stats(beatmap: &Beatmap, mods: GameMods) -> MapMaxStatsResult {
    let diff_attrs = rosu_pp::Difficulty::new()
        .mods(mods.clone())
        .calculate_for_mode::<Osu>(beatmap)
        .unwrap();

    let perf_attrs = rosu_pp::Performance::new(diff_attrs.clone())
        .mods(mods)
        .calculate();

    let stars = diff_attrs.stars;
    let combo = diff_attrs.max_combo;
    let pp = perf_attrs.pp();

    MapMaxStatsResult {
        perf_attrs,
        stars,
        combo,
        pp,
    }
}

pub fn calculate_nc_stats(perf_attrs: PerformanceAttributes, score: &Score) -> NoChokeStats {
    let stats = &score.statistics;
    let max_stats = &score.maximum_statistics;
    let is_classic = is_classic(&score.mods);

    let total_hits = stats.great + stats.ok + stats.meh;
    let ratio_300 = stats.great as f32 / total_hits as f32;
    let ratio_100 = stats.ok as f32 / total_hits as f32;

    let miss_to_300 = (stats.miss as f32 * ratio_300).round() as u32;
    let miss_to_100 = (stats.miss as f32 * ratio_100).round() as u32;
    let miss_to_50 = stats.miss - (miss_to_300 + miss_to_100);

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
        slider_end_hits: stats.slider_tail_hit,
        large_tick_hits: max_stats.large_tick_hit,
        small_tick_hits: max_stats.small_tick_hit,
        ..OsuScoreState::new()
    };

    let nc_acc = if is_classic {
        state.accuracy(OsuScoreOrigin::Stable) * 100.0
    } else {
        state.accuracy(OsuScoreOrigin::WithSliderAcc {
            max_large_ticks: max_stats.large_tick_hit,
            max_slider_ends: stats.slider_tail_hit,
        }) * 100.0
    };

    let nc_attrs = perf_attrs
        .performance()
        .mods(score.mods.clone())
        .n300(nc_300)
        .n100(nc_100)
        .n50(nc_50)
        .slider_end_hits(stats.slider_tail_hit)
        .lazer(!is_classic)
        .calculate();

    let nc_pp = nc_attrs.pp();

    NoChokeStats {
        perf_attrs: nc_attrs,
        n300: nc_300,
        n100: nc_100,
        n50: nc_50,
        miss: 0,
        acc: nc_acc,
        pp: nc_pp,
    }
}

pub struct NoChokeStats {
    pub perf_attrs: PerformanceAttributes,
    pub n300: u32,
    pub n100: u32,
    pub n50: u32,
    pub miss: u32,
    pub acc: f64,
    pub pp: f64,
}

pub struct MapMaxStatsResult {
    pub perf_attrs: PerformanceAttributes,
    pub stars: f64,
    pub combo: u32,
    pub pp: f64,
}

pub struct MapStats {
    pub ar: f32,
    pub od: f32,
    pub cs: f32,
    pub hp: f32,
    pub bpm: f32,
    pub seconds_drain: u32,
}
