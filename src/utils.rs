use crate::{
    Error, FAIL_EMBED_COLOR,
    resource_handler::{ResourceCategory, get_resource_path, remove_resource, save_resource},
};
use bytes::Bytes;
use num_format::{Locale, ToFormattedString};
use num_traits::{Float, clamp_min};
use poise::{Context as PoiseContext, CreateReply};
use rosu_pp::Beatmap;
use rosu_v2::prelude::{GameMod, GameMods};
use serenity::{
    Result as SerenityResult,
    all::{CreateAllowedMentions, CreateEmbed},
};
use std::{fs::File, io::Write, time::Duration};
use time::{OffsetDateTime, format_description};
use timeago::Formatter;

pub static BPM_EMOJI: &str = "<:bpm:1437855552100368384>";

pub async fn save_map_osu_file(map_id: u32) -> Result<String, Error> {
    let file_name = format!("{}.osu", map_id);

    if let Some(path) = get_resource_path(ResourceCategory::MapData, &file_name) {
        return Ok(path);
    }

    let map_data_url = map_osu_file_url(map_id);
    let map_response = reqwest::get(&map_data_url).await?;
    let map_data = map_response.bytes().await?;
    let path = save_resource(ResourceCategory::MapData, &file_name, map_data)?;
    Ok(path)
}

pub fn get_beatmap_locally(map_id: u32) -> Result<Beatmap, Error> {
    let file_name = format!("{}.osu", map_id);

    let path =
        get_resource_path(ResourceCategory::MapData, &file_name).ok_or("beatmap not found")?;

    rosu_pp::Beatmap::from_path(path).map_err(|err| {
        let _ = remove_resource(ResourceCategory::MapData, &file_name);
        format!("{}\nbeatmap file likely corrupted, file removed", err).into()
    })
}

pub fn map_osu_file_url(map_id: u32) -> String {
    format!("https://osu.ppy.sh/osu/{}", map_id)
}

pub fn format_hits(n300: u32, n100: u32, n50: u32, miss: u32) -> String {
    format!("{{{}/{}/{}/{}}}", n300, n100, n50, miss)
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

pub fn formatted_song_length(seconds: u32) -> String {
    let hour = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hour < 1 {
        format!("{}:{:02}", minutes, secs)
    } else {
        format!("{hour}:{:02}:{:02}", minutes, secs)
    }
}

pub fn save_file(bytes: Bytes, path: &str) -> Result<(), Error> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    Ok(())
}

pub fn discord_time_ago(time: OffsetDateTime) -> String {
    format!("<t:{}:R>", time.unix_timestamp())
}

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

pub fn player_not_found_embed(name: String) -> CreateEmbed {
    CreateEmbed::new()
        .color(FAIL_EMBED_COLOR)
        .description(format!("User {} was not found", wrap_in_tilde(name)))
}

pub fn failed_embed() -> CreateEmbed {
    CreateEmbed::new()
        .color(FAIL_EMBED_COLOR)
        .description("Something went wrong".to_string())
}

pub fn failed_embed_custom(custom_err: String) -> CreateEmbed {
    CreateEmbed::new()
        .color(FAIL_EMBED_COLOR)
        .description(custom_err)
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

pub fn wrap_in_tilde(text: String) -> String {
    format!("`{text}`")
}

pub fn playtime_in_hours(seconds: u32) -> String {
    if seconds == 0 {
        return "-".to_string();
    }
    (seconds as f32 / 3600.0).round().to_string() + "h"
}

pub fn check_reply(result: SerenityResult<poise::ReplyHandle<'_>>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

pub async fn reply_with_embed(ctx: &PoiseContext<'_, (), Error>, embed: CreateEmbed) {
    let embed_reply = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(CreateAllowedMentions::default().replied_user(false));

    check_reply(ctx.send(embed_reply).await);
}

pub trait CommaFormat {
    fn format(&self) -> String;
}

pub trait CommaFormatFloat {
    fn format(&self) -> String;
    fn two_decimal(&self) -> f32;
    fn format_acc(&self) -> String;
}

impl<T> CommaFormat for T
where
    T: ToFormattedString,
{
    fn format(&self) -> String {
        self.to_formatted_string(&Locale::en)
    }
}

impl<T> CommaFormatFloat for T
where
    T: ToString + Float,
{
    /// Formats a float into a string with comma-separated integer part and up to two decimals.
    fn format(&self) -> String {
        let integer_part = self
            .floor()
            .to_i32()
            .unwrap_or(0)
            .to_formatted_string(&Locale::en);

        let decimal_part = (self.fract().to_f32().unwrap_or(0.0) * 100.0).round() / 100.0;

        if decimal_part == 0.0 {
            return integer_part;
        }

        let mut formatted_decimals: String = decimal_part.to_string().chars().skip(1).collect();
        if formatted_decimals.len() == 2 {
            formatted_decimals.push('0');
        }

        format!("{}{}", integer_part, formatted_decimals)
    }

    fn two_decimal(&self) -> f32 {
        let num = self.to_f32().unwrap_or(0.0);
        (num * 100.0).round() / 100.0
    }

    fn format_acc(&self) -> String {
        format!("{}%", self.two_decimal())
    }
}
