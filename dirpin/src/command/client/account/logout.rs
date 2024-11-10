use dirpin_client::api_client::AuthClient;
use dirpin_client::settings::Settings;
use eyre::Result;

pub async fn run(settings: &Settings) -> Result<()> {
    let session = settings.session();

    if session.is_none() {
        println!("You are not logged in.");
        return Ok(());
    }

    // 2. api_client request logout.
    let client = AuthClient::new(&settings.server_address, &session.unwrap())?;
    let res = client.logout().await?;

    if !res.ok {
        println!("Remote server did not log out sessoin");
    }

    // 3. remove session file.
    fs_err::remove_file(&settings.session_path)?;

    // 4. Notify user that we are logged out.
   println!("You are logged out!");
    Ok(())
}
