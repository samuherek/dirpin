use crate::domain::HostId;
use config::builder::DefaultState;
use config::{Config, ConfigBuilder, Environment, File as ConfigFile, FileFormat};
use eyre::{eyre, Context, Result};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const EXAMPLE_CONFIG: &str = include_str!("../config.toml");
const HOST_ID_FILENAME: &str = "host_id";
const LAST_SYNC_FILENAME: &str = "last_sync_time";

// TODO: Research if storing the session and the key is ok in the
// files in the conifg. Maybe we need to use the OS secret storage?
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Settings {
    pub db_path: String,
    pub key_path: String,
    pub session_path: String,
    pub server_address: String,
}

impl Settings {
    fn read_from_data_dir(filename: &str) -> Option<String> {
        let data_dir = dirpin_common::utils::data_dir();
        let path = data_dir.join(filename);

        if !path.exists() {
            return None;
        }

        let value = fs_err::read_to_string(path);
        value.ok()
    }

    fn save_to_data_dir(filename: &str, value: &str) -> Result<()> {
        let data_dir = dirpin_common::utils::data_dir();
        let path = data_dir.join(filename);
        fs_err::write(path, value)?;
        Ok(())
    }

    pub fn save_last_sync() -> Result<()> {
        Settings::save_to_data_dir(
            LAST_SYNC_FILENAME,
            OffsetDateTime::now_utc().format(&Rfc3339)?.as_str(),
        )?;
        Ok(())
    }

    pub fn last_sync() -> Result<OffsetDateTime> {
        let value = Settings::read_from_data_dir(LAST_SYNC_FILENAME);
        match value {
            Some(v) => Ok(OffsetDateTime::parse(&v, &Rfc3339)?),
            None => Ok(OffsetDateTime::UNIX_EPOCH),
        }
    }

    pub fn host_id() -> HostId {
        let id = Settings::read_from_data_dir(HOST_ID_FILENAME);
        if let Some(id) = id {
            let host_id = HostId::from_str(id.as_str()).expect("Failed to parse local host id");
            host_id
        } else {
            let host_id = HostId::gen_host_id();
            Settings::save_to_data_dir(HOST_ID_FILENAME, host_id.as_ref())
                .expect("Failed to write local host id");
            host_id
        }
    }

    // TODO Make the String into a SessionToken
    pub fn session(&self) -> Option<String> {
        let path = PathBuf::from(&self.session_path);

        if !path.exists() {
            return None;
        }

        let value = fs_err::read_to_string(path);
        value.ok()
    }

    pub fn builder() -> Result<ConfigBuilder<DefaultState>> {
        let data_dir = dirpin_common::utils::data_dir();
        let db_path = data_dir.join("pins.db");
        let key_path = data_dir.join("key");
        // TODO: make the sessions path and the host_id path consistent. They are kind of private
        // for the local computer but at the same time it would be nice to have on the settings.
        // However, we don't really want the user to overwrite it :|
        let session_path = data_dir.join("session");

        Ok(Config::builder()
            .set_default("db_path", db_path.to_str())?
            .set_default("key_path", key_path.to_str())?
            .set_default("session_path", session_path.to_str())?
            .set_default("server_address", "http://127.0.0.1:8090")?
            .add_source(
                Environment::with_prefix("dirpin")
                    .prefix_separator("_")
                    .separator("__"),
            ))
    }

    pub fn new() -> Result<Self> {
        let config_dir = dirpin_common::utils::config_dir();
        let data_dir = dirpin_common::utils::data_dir();

        create_dir_all(&config_dir)
            .wrap_err_with(|| format!("Failed to create dir {config_dir:?}"))?;
        create_dir_all(&data_dir).wrap_err_with(|| format!("Failed to create dir {data_dir:?}"))?;

        let mut config_file = if let Ok(p) = std::env::var("DIRPIN_CONFIG_DIR") {
            PathBuf::from(p)
        } else {
            let mut config_file = PathBuf::new();
            config_file.push(config_dir);
            config_file
        };

        config_file.push("config.toml");

        let mut config_builder = Self::builder()?;
        config_builder = if config_file.exists() {
            config_builder.add_source(ConfigFile::new(
                config_file.to_str().unwrap(),
                FileFormat::Toml,
            ))
        } else {
            let mut file = File::create(config_file).wrap_err("Failed to create config file")?;
            file.write_all(EXAMPLE_CONFIG.as_bytes())
                .wrap_err("Failed to write default config file")?;
            config_builder
        };

        let mut settings: Settings = config_builder
            .build()?
            .try_deserialize()
            .map_err(|e| eyre!("Failed to deserialize {}", e))?;

        settings.db_path = expand_shell(&settings.db_path)?;
        settings.key_path = expand_shell(&settings.key_path)?;
        settings.session_path = expand_shell(&settings.session_path)?;

        Ok(settings)
    }
}

fn expand_shell(value: &str) -> Result<String> {
    Ok(shellexpand::full(value)?.to_string())
}
