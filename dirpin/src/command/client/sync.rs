use dirpin_client::settings::Settings;
use eyre::Result;

pub(crate) async fn run(settings: &Settings) -> Result<()> {
    // Load the last sync timestamp
    //
    // Request if new changes from the server since the last timestamp -> Download
    // If new changes between last sync and now locally -> Upload
    dirpin_client::sync::sync(settings).await?;
    Ok(())
}
