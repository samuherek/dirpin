use crate::api_client;
use crate::settings::Settings;
use eyre::Result;

pub async fn sync(settings: &Settings) -> Result<()> {
    api_client::handle_sync(&settings.server_address).await?;
    Ok(())
}
