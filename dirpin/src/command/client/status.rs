use dirpin_client::api_client;
use dirpin_client::settings::Settings;
use eyre::Result;

pub async fn run(settings: &Settings) -> Result<()>{
    let res = api_client::health_check(&settings.server_address).await?;
    println!("{res:?}");
    Ok(())
}
