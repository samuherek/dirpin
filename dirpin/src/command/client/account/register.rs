use clap::Parser;
use dirpin_client::settings::Settings;
use dirpin_client::utils::{read_input, read_input_hidden};
use dirpin_client::{api_client, encryption};
use eyre::{Context, Result};
use fs_err;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {
    #[arg(short, long)]
    username: Option<String>,
    #[arg(short, long)]
    email: Option<String>,
    #[arg(short, long)]
    password: Option<String>,
}

impl Cmd {
    pub async fn run(self, settings: &Settings) -> Result<()> {
        // get the inputs or ask the user to type it in
        let username = self.username.unwrap_or_else(|| read_input("username"));
        let email = self.email.unwrap_or_else(|| read_input("email"));
        let password = self
            .password
            .unwrap_or_else(|| read_input_hidden("password"));

        // use the dirpin-client to request the register and return session
        let res = api_client::register(&settings.server_address, &username, &email, &password)
            .await
            .wrap_err("Failed to register user")?;

        // save the session in the file
        let session_path = PathBuf::from(&settings.session_path);
        fs_err::write(session_path, res.session.as_bytes())
            .wrap_err("Failed to store session in file")?;

        // make sure the "key" is loaded
        let _key = encryption::load_key(settings)?;

        Ok(())
    }
}
