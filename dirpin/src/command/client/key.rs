use dirpin_client::encryption;
use dirpin_client::settings::Settings;
use eyre::Result;

pub fn run(settings: &Settings) -> Result<()> {
    let key = encryption::load_key(settings)?;
    println!("key after load: {key:?}");
    Ok(())
}
