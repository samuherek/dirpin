use dirpin_client::settings::Settings;
use eyre::Result;
use std::path::PathBuf;

pub async fn run(settings: &Settings) -> Result<()> {
    let session_path = PathBuf::from(&settings.session_path);

    if !session_path.exists() {
        println!("You are not logged in.");
        return Ok(());
    }

    // 1. check if session exists. Otherwise log that we are not logged in.
    // 2. api_client request logout.
    // 3. remove session file.
    // 4. Notify user that we are logged out.

    Ok(())
}
