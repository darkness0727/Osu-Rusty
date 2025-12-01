pub mod error;
pub mod profile;
pub mod recent;
pub mod score;

pub static FAIL_EMBED_COLOR: i32 = 0xFF0000;
pub static DEFAULT_EMBED_COLOR: i32 = 0x699BC7;

pub static PP_GAINED_TEXT: &str = "**[(?)](https://discord.com/channels/1297750821219467264/1297838959854096454/# \"the amount of raw profile PP gained from this play accounting for previous scores on the map, this does not include bonus PP and the value is only accurate if this is the most recent top play\")**";
pub static MISSING_TEXT: &str = "**[(?)](https://discord.com/channels/1297750821219467264/1297838959854096454/# \"the top200 did not include this score likely because the api wasn't done processing but presumably the score is in there\")**";
