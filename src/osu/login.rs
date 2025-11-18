use rosu_v2::prelude::*;

pub async fn login(client_id: u64, client_secret: String) -> Result<Osu, OsuError> {
    Osu::new(client_id, client_secret).await
}
