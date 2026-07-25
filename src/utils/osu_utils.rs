use std::{collections::HashMap, sync::{LazyLock, Mutex}, time::Duration};

use crate::{
    Error, OSU_CLIENT,
    resource_handler::{ResourceCategory, get_resource_path, save_resource},
    utils::osu_pp::slider_tail_tick_miss,
};
use num_traits::clamp_min;
use regex::Regex;
use reqwest::Url;
use rosu_pp::Beatmap;
use rosu_v2::{
    error::OsuError, model::Grade, prelude::{BeatmapExtended, BeatmapsetExtended, Score, UserExtended}, request::UserId,
};
use time::{OffsetDateTime, format_description};
use timeago::Formatter;

pub static MAX_TOP_PLAY_COUNT: usize = 200;

static DIGIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").unwrap());

static BEATMAP_CACHE: LazyLock<Mutex<HashMap<u32, Beatmap>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn login(client_id: u64, client_secret: String) -> Result<rosu_v2::Osu, OsuError> {
    rosu_v2::Osu::new(client_id, client_secret).await
}

pub async fn fetch_player(user: impl Into<UserId>) -> Result<UserExtended, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();
    osu.user(user).await
}

pub async fn fetch_recent_scores(
    user: impl Into<UserId>,
    amount: usize,
    include_false: bool,
) -> Result<Vec<Score>, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.user_scores(user)
        .recent()
        .limit(amount)
        .include_fails(include_false)
        .await
}

pub async fn fetch_map_scores(user: impl Into<UserId>, map_id: u32) -> Result<Vec<Score>, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.beatmap_user_scores(map_id, user).await
}

pub async fn fetch_map(map_id: u32) -> Result<BeatmapExtended, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.beatmap().map_id(map_id).await
}

pub async fn fetch_mapset(mapset_id: u32) -> Result<BeatmapsetExtended, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.beatmapset(mapset_id).await
}

pub async fn fetch_mapset_from_diff(map_id: u32) -> Result<BeatmapsetExtended, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.beatmapset_from_map_id(map_id).await
}

pub async fn fetch_personal_bests(
    user: impl Into<UserId>,
    amount: usize,
    offset: usize,
) -> Result<Vec<Score>, Error> {
    let osu = OSU_CLIENT.get().unwrap();
    let max_per_call = 100;
    let user: UserId = user.into();

    if amount <= max_per_call {
        osu.user_scores(user)
            .best()
            .limit(amount)
            .offset(offset)
            .await
            .map_err(Into::into)
    } else {
        let top_plays_handle = tokio::spawn(
            osu.user_scores(user.clone())
                .best()
                .limit(max_per_call)
                .offset(offset)
                .into_future(),
        );

        let second_limit = amount - max_per_call;
        let second_offset = offset + max_per_call;

        let top_plays_handle_2 = tokio::spawn(
            osu.user_scores(user)
                .best()
                .limit(second_limit)
                .offset(second_offset)
                .into_future(),
        );

        let mut top_plays = top_plays_handle
            .await
            .map_err(|e| -> Error { e.into() })??;
        let top_plays_2 = top_plays_handle_2
            .await
            .map_err(|e| -> Error { e.into() })??;

        top_plays.extend(top_plays_2);
        Ok(top_plays)
    }
}

pub async fn download_map_file(map_id: u32) -> Result<String, Error> {
    let file_user = format!("{}.osu", map_id);

    if let Some(path) = get_resource_path(ResourceCategory::MapData, &file_user) {
        return Ok(path);
    }

    let map_data_url = format!("https://osu.ppy.sh/osu/{}", map_id);
    let map_response = reqwest::get(&map_data_url).await?;
    let map_data = map_response.bytes().await?;
    let path = save_resource(ResourceCategory::MapData, &file_user, map_data)?;
    Ok(path)
}

pub fn load_local_beatmap(map_id: u32) -> Result<Beatmap, Error> {
    {
        let cache = BEATMAP_CACHE.lock().unwrap();
        if let Some(map) = cache.get(&map_id) {
            return Ok(map.clone());
        }
    }

    let file_user = format!("{}.osu", map_id);

    let path =
        get_resource_path(ResourceCategory::MapData, &file_user).ok_or("beatmap not found")?;

    let map = rosu_pp::Beatmap::from_path(path)?;
    map.check_suspicion()?;

    BEATMAP_CACHE.lock().unwrap().insert(map_id, map.clone());

    Ok(map)
}

pub fn format_hits(n300: u32, n100: u32, n50: u32, miss: u32) -> String {
    format!("{{{}/{}/{}/{}}}", n300, n100, n50, miss)
}

/// ChatGPT wrote this not me
pub fn parse_beatmap_url(s: &str) -> ParsedUrlResult {
    let trimmed = s.trim();

    // 0) If the entire input is a number -> treat as map_id
    if let Ok(n) = trimmed.parse::<u32>() {
        return ParsedUrlResult {
            mapset_id: None,
            map_id: Some(n),
        };
    }

    // Make it a valid URL if it isn't one already (adds https://)
    let candidate = if Url::parse(trimmed).is_ok() {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    };

    // If still cannot parse, return empty result (per your note).
    let url = match Url::parse(&candidate) {
        Ok(u) => u,
        Err(_) => {
            return ParsedUrlResult {
                mapset_id: None,
                map_id: None,
            };
        }
    };

    let mut mapset_id: Option<u32> = None;
    let mut map_id: Option<u32> = None;

    // 1) Fragment (prefer this for map_id), e.g. "#osu/4924798" -> 4924798
    if let Some(fragment) = url.fragment()
        && let Some(mat) = DIGIT_RE.find_iter(fragment).last() {
            map_id = mat.as_str().parse().ok();
        }

    // 2) Path segments — explicit patterns first
    if let Some(segments) = url.path_segments() {
        let segs: Vec<&str> = segments.filter(|seg| !seg.is_empty()).collect();

        for i in 0..segs.len() {
            match segs[i] {
                "beatmapsets" if i + 1 < segs.len() => {
                    if mapset_id.is_none() {
                        mapset_id = segs[i + 1].parse::<u32>().ok();
                    }
                }
                "beatmaps" | "beatmap" | "b" if i + 1 < segs.len()
                    && map_id.is_none() => {
                        map_id = segs[i + 1].parse::<u32>().ok();
                    }
                _ => {}
            }
        }

        // 2b) Path fallback: consider last numeric token in the path only in safe cases
        if map_id.is_none() {
            // collect all numeric tokens in the path
            let path_digits: Vec<u32> = DIGIT_RE
                .find_iter(url.path())
                .filter_map(|m| m.as_str().parse::<u32>().ok())
                .collect();

            if !path_digits.is_empty() {
                if path_digits.len() > 1 {
                    // multiple numbers in path -> last one is likely the map_id
                    map_id = path_digits.last().cloned();
                } else {
                    // exactly one numeric token in path:
                    // only treat it as map_id if we didn't already identify it as mapset_id
                    let only = path_digits[0];
                    if mapset_id.is_none() {
                        // no mapset found -> this single number is probably a map_id (e.g. /beatmap/4924798)
                        map_id = Some(only);
                    } else {
                        // mapset_id exists and it's the only number in path -> do NOT set map_id
                        // (this prevents treating /beatmapsets/2285243 as map_id)
                    }
                }
            }
        }
    }

    // 3) Query fallback (e.g. ?id=4924798), only if map_id still missing
    if map_id.is_none()
        && let Some(q) = url.query()
            && let Some(mat) = DIGIT_RE.find_iter(q).last() {
                map_id = mat.as_str().parse().ok();
            }

    ParsedUrlResult { mapset_id, map_id }
}

/// ALSO written by ChatGPT rest in peace
pub fn highest_pp_score(mut scores: Vec<Score>) -> Option<(Score, Vec<Score>)> {
    if scores.is_empty() {
        return None;
    }

    // Find index of highest-pp score
    let best_idx = scores
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.pp.map(|pp| (pp, i)))
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .map(|(_, idx)| idx)
        .unwrap_or(0);

    // Remove it from the vec
    let picked = scores.remove(best_idx);

    Some((picked, scores))
}

pub struct ParsedUrlResult {
    pub mapset_id: Option<u32>,
    pub map_id: Option<u32>,
}

pub fn format_slider_misses(score: &Score, map: &Beatmap) -> Option<String> {
    let stats = slider_tail_tick_miss(score, map)?;

    let tick_miss = stats.tick_miss;
    let tail_miss = stats.tail_miss;

    let tick_miss_text = format_slider_tick_misses_from_stats(tick_miss);
    let tail_miss_text = if tail_miss > 0 { format!("{tail_miss}{TAIL_MISS_EMOJI}") } else { Default::default() };

    let has_any = !tick_miss_text.is_empty() || !tail_miss_text.is_empty();
    has_any.then(|| format!("{tick_miss_text}{tail_miss_text}"))
}

pub fn format_slider_tick_misses(score: &Score, map: &Beatmap) -> Option<String> {
    let stats = slider_tail_tick_miss(score, map)?;
    let text = format_slider_tick_misses_from_stats(stats.tick_miss);
    (!text.is_empty()).then_some(text)
}

fn format_slider_tick_misses_from_stats(tick_miss: u32) -> String {
    if tick_miss > 0 { format!("{tick_miss}{TICK_MISS_EMOJI}") } else { Default::default() }
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

pub static MISS_EMOJI: &str = "<:miss:1445495578862817280>";
pub static BPM_EMOJI: &str = "<:bpm:1437855552100368384>";
pub static TICK_MISS_EMOJI: &str = "<:slider_tick_miss:1441484864049123399>";
pub static TAIL_MISS_EMOJI: &str = "<:slider_tail_miss:1441692017775083642>";

pub fn grade_emoji(grade: Grade) -> String {
    match grade {
        Grade::X => "<:SS:1346458936596889640>",
        Grade::S => "<:S_:1346458998425128990>",
        Grade::XH => "<:SSH:1346459029656047646>",
        Grade::SH => "<:SH:1346459119741046794>",
        Grade::A => "<:A_:1346459159935193139>",
        Grade::B => "<:B_:1346459185512054814>",
        Grade::C => "<:C_:1346459204847796264>",
        Grade::D => "<:D_:1347295031756587039>",
        Grade::F => "<:F_:1346460123173879859>",
    }
    .to_string()
}

pub fn format_join_date(date: OffsetDateTime) -> String {
    let format = format_description::parse_borrowed::<1>(
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
