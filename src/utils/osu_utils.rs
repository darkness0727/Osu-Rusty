use std::{ops::Not, time::Duration};

use crate::{
    Error, OSU_CLIENT,
    resource_handler::{ResourceCategory, get_resource_path, save_resource},
    utils::CommaFormatFloat,
};
use num_traits::clamp_min;
use rosu_pp::{
    Beatmap,
    any::PerformanceAttributes,
    osu::{Osu as Osu_Pp, OsuScoreOrigin, OsuScoreState},
};
use rosu_v2::{
    error::OsuError,
    prelude::{GameMod, GameMods, Score, UserExtended},
};
use time::{OffsetDateTime, format_description};
use timeago::Formatter;

pub async fn login(client_id: u64, client_secret: String) -> Result<rosu_v2::Osu, OsuError> {
    rosu_v2::Osu::new(client_id, client_secret).await
}

pub async fn fetch_player(name: String) -> Result<UserExtended, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();
    osu.user(&name).await
}

pub async fn fetch_recent_scores(name: String, amount: usize) -> Result<Vec<Score>, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.user_scores(&name).recent().limit(amount).await
}

pub async fn fetch_personal_bests(
    name: String,
    amount: usize,
    offset: usize,
) -> Result<Vec<Score>, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.user_scores(&name)
        .best()
        .limit(amount)
        .offset(offset)
        .await
}

/// Returns if the score is present in the top200 and its index. if the PP is high enough be present in the top200, the index.
pub async fn is_in_pb(top_plays: Vec<Score>, score: &Score) -> Result<IsPbResult, Error> {
    let top_ids  = top_plays.iter().map(|s| s.id);
    let top_pps = top_plays.iter().map(|s| s.pp.unwrap_or(0.0));

    let mut score_pp = score.pp.unwrap_or_default();

    if score.pp.is_none() {
        download_map_file(score.map_id).await?;
        let beatmap = load_local_beatmap(score.map_id)?;
        score_pp = cal_score_pp_beatmap(&beatmap, score) as f32;
    } else {
        // check for score ID match
        for (index, id) in top_ids.enumerate() {
            if id == score.id {
                return Ok(IsPbResult::InPB(index + 1));
            }
        }

        // return NotPB if there is a different score on the same map with a higher PP
        for top_score in top_plays.iter() {
            if top_score.map_id == score.map_id && top_score.pp.unwrap_or(0.0) >= score_pp {
                return Ok(IsPbResult::NotPB);
            }
        }
    }

    // return what index the play would be if it's pp is high enough to be a top200 score
    for (index, pp) in top_pps.enumerate() {
        if score_pp > pp {
            if score.pp.is_none() {
                return Ok(IsPbResult::IfRanked(index + 1));
            }
            {
                return Ok(IsPbResult::MissingPB(index + 1));
            }
        }
    }

    Ok(IsPbResult::NotPB)
}

pub enum IsPbResult {
    InPB(usize),
    MissingPB(usize),
    IfRanked(usize),
    NotPB,
}

pub fn is_fc(score: &Score, map_combo: u32, slider_count: u32) -> bool {
    if !is_classic(&score.mods) {
        let stats = &score.statistics;
        return stats.miss + stats.small_tick_miss + stats.large_tick_miss == 0;
    }

    let fc_threshold = map_combo as f32 - 0.1 * slider_count as f32;
    score.max_combo as f32 >= fc_threshold
}

pub fn cal_score_pp_beatmap(beatmap: &Beatmap, score: &Score) -> f64 {
    let stats = &score.statistics;
    let is_classic = is_classic(&score.mods);

    let mods = score.mods.clone();

    let diff_attrs = rosu_pp::Difficulty::new()
        .mods(mods.clone())
        .calculate_for_mode::<Osu_Pp>(beatmap)
        .unwrap();

    let perf_attrs = rosu_pp::Performance::new(diff_attrs).mods(mods).calculate();

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

    let slider_tail_miss = is_classic
        .not()
        .then(|| max_stats.slider_tail_hit - stats.slider_tail_hit)
        .unwrap_or(0);

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

pub async fn download_map_file(map_id: u32) -> Result<String, Error> {
    let file_name = format!("{}.osu", map_id);

    if let Some(path) = get_resource_path(ResourceCategory::MapData, &file_name) {
        return Ok(path);
    }

    let map_data_url = format!("https://osu.ppy.sh/osu/{}", map_id);
    let map_response = reqwest::get(&map_data_url).await?;
    let map_data = map_response.bytes().await?;
    let path = save_resource(ResourceCategory::MapData, &file_name, map_data)?;
    Ok(path)
}

pub fn load_local_beatmap(map_id: u32) -> Result<Beatmap, Error> {
    let file_name = format!("{}.osu", map_id);

    let path =
        get_resource_path(ResourceCategory::MapData, &file_name).ok_or("beatmap not found")?;

    let map = rosu_pp::Beatmap::from_path(path)?;
    map.check_suspicion()?;

    Ok(map)
}

pub fn format_hits(n300: u32, n100: u32, n50: u32, miss: u32) -> String {
    format!("{{{}/{}/{}/{}}}", n300, n100, n50, miss)
}

pub fn format_slider_misses(score: &Score) -> Option<String> {
    if is_classic(&score.mods) {
        return None;
    };

    let stats = &score.statistics;
    let tick_miss = stats.small_tick_miss + stats.large_tick_miss;
    let tail_miss = score.maximum_statistics.slider_tail_hit - stats.slider_tail_hit;

    let has_tick_miss = tick_miss > 0;
    let has_tail_miss = tail_miss > 0;

    if !has_tick_miss && !has_tail_miss {
        return None;
    };

    let tick_miss_text = has_tick_miss
        .then(|| format!("**{tick_miss}**{TICK_MISS_EMOJI}"))
        .unwrap_or_default();

    let tail_miss_text = has_tail_miss
        .then(|| format!("**{tail_miss}**{TAIL_MISS_EMOJI}"))
        .unwrap_or_default();

    Some(format!("{tick_miss_text}{tail_miss_text}"))
}

pub fn is_classic(mods: &GameMods) -> bool {
    mods.iter().any(|m| matches!(m, &GameMod::ClassicOsu(_)))
}

/// Color spectrum interpolation for star rating.
/// This function was written by ChatGPT and I have ZERO idea of
/// whats actually happening but if its works it works
pub fn star_color_spectrum(stars: f32) -> i32 {
    const D: [f32; 11] = [0.1, 1.25, 2.0, 2.5, 3.3, 4.2, 4.9, 5.8, 6.7, 7.7, 9.0];
    const C: [(u8, u8, u8); 11] = [
        (0x42, 0x90, 0xFB),
        (0x4F, 0xC0, 0xFF),
        (0x4F, 0xFF, 0xD5),
        (0x7C, 0xFF, 0x4F),
        (0xF6, 0xF0, 0x5C),
        (0xFF, 0x80, 0x68),
        (0xFF, 0x4E, 0x6F),
        (0xC6, 0x45, 0xB8),
        (0x65, 0x63, 0xDE),
        (0x18, 0x15, 0x8E),
        (0x00, 0x00, 0x01),
    ];

    let s = stars.clamp(D[0], D[10]);

    let mut i = 0usize;
    for idx in 0..(D.len() - 1) {
        if s >= D[idx] && s <= D[idx + 1] {
            i = idx;
            break;
        }
    }

    let denom = D[i + 1] - D[i];
    let t = if denom == 0.0 {
        0.0
    } else {
        (s - D[i]) / denom
    };

    let r = (C[i].0 as f32 + (C[i + 1].0 as f32 - C[i].0 as f32) * t).round() as i32;
    let g = (C[i].1 as f32 + (C[i + 1].1 as f32 - C[i].1 as f32) * t).round() as i32;
    let b = (C[i].2 as f32 + (C[i + 1].2 as f32 - C[i].2 as f32) * t).round() as i32;

    (r << 16) | (g << 8) | b
}

pub fn formated_song_length(seconds: u32) -> String {
    let hour = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hour < 1 {
        format!("{}:{:02}", minutes, secs)
    } else {
        format!("{hour}:{:02}:{:02}", minutes, secs)
    }
}

pub fn relative_timestamp(time: OffsetDateTime) -> String {
    format!("<t:{}:R>", time.unix_timestamp())
}

pub static BPM_EMOJI: &str = "<:bpm:1437855552100368384>";
pub static TICK_MISS_EMOJI: &str = "<:slider_tick_miss:1441484864049123399>";
pub static TAIL_MISS_EMOJI: &str = "<:slider_tail_miss:1441484819698815129>";

pub fn grade_emoji(grade: String) -> String {
    match grade.to_uppercase().as_str() {
        "SS" => "<:SS:1346458936596889640>".to_string(),
        "S" => "<:S_:1346458998425128990>".to_string(),
        "XH" => "<:SSH:1346459029656047646>".to_string(),
        "SH" => "<:SH:1346459119741046794>".to_string(),
        "A" => "<:A_:1346459159935193139>".to_string(),
        "B" => "<:B_:1346459185512054814>".to_string(),
        "C" => "<:C_:1346459204847796264>".to_string(),
        "D" => "<:D_:1347295031756587039>".to_string(),
        "F" => "<:F_:1346460123173879859>".to_string(),
        _ => "invalid_grade".to_string(),
    }
}

pub fn format_join_date(date: OffsetDateTime) -> String {
    let format = format_description::parse(
        "Joined on [day padding:none] [month repr:long padding:none] [year] at [hour repr:12 padding:none]:[minute] [period case:lower] UTC +0",
    )
    .unwrap_or_default();
    let formated_date = date.format(&format).unwrap_or_default();

    let now = OffsetDateTime::now_utc();

    let seconds_since_joined = clamp_min((now - date).whole_seconds(), 0) as u64;
    let ago = Formatter::new().convert(Duration::from_secs(seconds_since_joined));

    format!("{formated_date} ({ago})")
}

pub fn get_flag_url(country_code: String, size: u16) -> String {
    format!("https://osuflags.omkserver.nl/{country_code}-{size}.png")
}

pub fn playtime_in_hours(seconds: u32) -> String {
    if seconds == 0 {
        return "-".to_string();
    }
    (seconds as f32 / 3600.0).round().to_string() + "h"
}
