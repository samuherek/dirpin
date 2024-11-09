use clap::Parser;
use dirpin_client::encryption;
use dirpin_client::settings::Settings;
use dirpin_client::utils::{read_input, read_input_hidden};
use eyre::{bail, Context, Result};
use fs_err;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {
    #[arg(long, short)]
    pub username: Option<String>,
    #[arg(long, short)]
    pub password: Option<String>,
    /// The encryption key for computer to decrypt remote data
    #[arg(long, short)]
    pub key: Option<String>,
}

impl Cmd {
    // TODO: make this into an encoded value that you can pass that will hold the username,
    // password and the key for easier login
    pub async fn run(self, settings: &Settings) -> Result<()> {
        let session_path = PathBuf::from(&settings.session_path);

        if session_path.exists() {
            println!("You are already logged in.");
            return Ok(());
        }

        // Try to get the username and password and the key
        let username = self.username.unwrap_or_else(|| read_input("username"));
        let password = self
            .password
            .unwrap_or_else(|| read_input_hidden("password"));
        // NOTE: do we need to hide this key in the temrinal?
        // Maybe it might be a good idea to create a corssterm interactive one liner instead?
        let key = self
            .key
            .unwrap_or_else(|| read_input("key [to use exiting key leave empty]"));
        let key_path = PathBuf::from(&settings.key_path);

        if key.is_empty() {
            // If key is empty, check if it exists and is valid in a key file. If not, ask the user if
            // to create a new key. This is incase the user just accidentally presses enter while
            // logging in on a new computer.
            if key_path.exists() {
                match encryption::read_key(&key_path) {
                    Ok(_) => {}
                    Err(_) => bail!("Failed to read local key from file"),
                }
            } else {
                println!("You have not provided a key and we could not find file key.");
                println!("Do you want to create a new key?");
                todo!("create new key");
            }
        } else {
            // The user provided a key. However, we need to make sure it's not trying to overwrite
            // the existing key. So we try to load the key file and if we can't find it, then
            // save this key. This happens for the case when the user is looging in to a new
            // compter with an existing account.
            if !key_path.exists() {
                // You have provided a key and the key path does not exist
                match encryption::decode_key(key.clone()) {
                    Ok(_) => fs_err::write(key_path, key).wrap_err("Failed to write key.")?,
                    Err(_) => bail!("Provided key seems to be invalid"),
                }
            } else {
                // Make sure to compare the provided and the local key from the key file.
                // We need to make sure that we ask the user if they are sure to overwrite
                // the key. As this would require a re-encryption of the data remotely.
                let local_key: [u8; 32] = encryption::read_key(&key_path)?.into();
                let provided_key: [u8; 32] = encryption::decode_key(key)?.into();

                if local_key != provided_key {
                    println!("You already have a key locally and you provided a different key.");
                    println!("Do you want to use the new key and re-encrypt the data?");
                    todo!("re-encrypt remote data");
                }
            }
        }

        // Get the session
        // - user must have valid username and password
        // - how do we make sure that we have correct key? Only on download and decryption
        // -
        let res = dirpin_client::api_client::login(
            settings.server_address.as_str(),
            username.as_str(),
            password.as_str(),
            Settings::host_id().as_ref(),
        )
        .await?;

        fs_err::write(settings.session_path.as_str(), res.session.as_bytes())
            .wrap_err("Failed to create a session file")?;

        println!("Logged in!");

        Ok(())
    }
}
