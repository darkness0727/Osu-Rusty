use std::time::Duration;

use crate::{
    Error, OSU_CLIENT,
    resource_handler::{ResourceCategory, get_resource_path, save_resource},
    utils::osu_pp::slider_tail_tick_miss,
};
use num_traits::clamp_min;
use rosu_pp::Beatmap;
use rosu_v2::{
    error::OsuError,
    model::Grade,
    prelude::{Score, UserExtended},
};
use time::{OffsetDateTime, format_description};
use timeago::Formatter;

pub static MAX_TOP_PLAY_COUNT: usize = 200;

pub async fn login(client_id: u64, client_secret: String) -> Result<rosu_v2::Osu, OsuError> {
    rosu_v2::Osu::new(client_id, client_secret).await
}

pub async fn fetch_player(name: String) -> Result<UserExtended, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();
    osu.user(&name).await
}

pub async fn fetch_recent_scores(
    name: String,
    amount: usize,
    include_false: bool,
) -> Result<Vec<Score>, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.user_scores(&name)
        .recent()
        .limit(amount)
        .include_fails(include_false)
        .await
}

pub async fn fetch_map_scores(name: String, map_id: u32) -> Result<Vec<Score>, OsuError> {
    let osu = OSU_CLIENT.get().unwrap();

    osu.beatmap_user_scores(map_id, name).await
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

pub async fn fetch_all_personal_bests(name: String) -> Result<Vec<Score>, Error> {
    let osu = OSU_CLIENT.get().unwrap();

    let top_plays_handle = tokio::spawn(
        osu.user_scores(&name)
            .best()
            .limit(100)
            .offset(0)
            .into_future(),
    );
    let top_plays_handle2 = tokio::spawn(
        osu.user_scores(&name)
            .best()
            .limit(100)
            .offset(100)
            .into_future(),
    );

    let mut top_plays = top_plays_handle.await??;
    let top_plays_second = top_plays_handle2.await??;

    top_plays.extend(top_plays_second);

    Ok(top_plays)
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

pub fn format_slider_misses(score: &Score, map: Beatmap) -> Option<String> {
    let stats = slider_tail_tick_miss(score, &map)?;

    let tick_miss = stats.tick_miss;
    let tail_miss = stats.tail_miss;

    let has_tick_miss = tick_miss > 0;
    let has_tail_miss = tail_miss > 0;

    if !has_tick_miss && !has_tail_miss {
        return None;
    };

    let tick_miss_text = has_tick_miss
        .then(|| format!("{tick_miss}{TICK_MISS_EMOJI}"))
        .unwrap_or_default();

    let tail_miss_text = has_tail_miss
        .then(|| format!("{tail_miss}{TAIL_MISS_EMOJI}"))
        .unwrap_or_default();

    Some(format!("{tick_miss_text}{tail_miss_text}"))
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
pub static TAIL_MISS_EMOJI: &str = "<:slider_tail_miss:1441692017775083642>";

pub fn grade_emoji(grade: Grade) -> String {
    match grade {
        Grade::X => String::from("<:SS:1346458936596889640>"),
        Grade::S => String::from("<:S_:1346458998425128990>"),
        Grade::XH => String::from("<:SSH:1346459029656047646>"),
        Grade::SH => String::from("<:SH:1346459119741046794>"),
        Grade::A => String::from("<:A_:1346459159935193139>"),
        Grade::B => String::from("<:B_:1346459185512054814>"),
        Grade::C => String::from("<:C_:1346459204847796264>"),
        Grade::D => String::from("<:D_:1347295031756587039>"),
        Grade::F => String::from("<:F_:1346460123173879859>"),
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
