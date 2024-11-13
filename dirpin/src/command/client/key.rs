use dirpin_client::encryption;
use dirpin_client::settings::Settings;
use eyre::Result;

pub fn run(settings: &Settings) -> Result<()> {
    let key = encryption::load_key(settings)?;
    let key = encryption::encode_key(&key)?;
    println!("{key}");
    Ok(())
}
