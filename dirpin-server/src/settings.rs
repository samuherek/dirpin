use config::builder::DefaultState;
use config::{Config, ConfigBuilder, Environment, File as ConfigFile, FileFormat};
use dirpin_common::utils::{config_dir, data_dir};
use eyre::{eyre, Error, Result};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

const EXAMPLE_CONFIG: &str = include_str!("../server.toml");

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub db_path: String,
}

impl Settings {
    pub fn new() -> Result<Self, Error> {
        let mut config_builder = Self::build_default()?;

        let config_dir = std::env::var("DIRPIN_CONFIG_DIR").map_or(config_dir(), PathBuf::from);
        let config_file = config_dir.join("server.toml");

        if config_file.exists() {
            config_builder = config_builder.add_source(ConfigFile::new(
                config_file.to_str().unwrap(),
                FileFormat::Toml,
            ));
        } else {
            create_dir_all(config_file.parent().unwrap())?;
            let mut file = File::create(config_file)?;
            file.write_all(EXAMPLE_CONFIG.as_bytes())?;
        };

        let settings = config_builder
            .build()?
            .try_deserialize()
            .map_err(|e| eyre!("Failed to deserialize config {}", e))?;

        Ok(settings)
    }

    pub fn build_default() -> Result<ConfigBuilder<DefaultState>> {
        let data_dir = std::env::var("DIRPIN_DATA_DIR").map_or(data_dir(), PathBuf::from);
        let db_path = data_dir.join("server.db");

        Ok(Config::builder()
            .set_default("host", "127.0.0.1")?
            .set_default("port", 8090)?
            .set_default("db_path", db_path.to_str())?
            .add_source(
                Environment::with_prefix("dirpin")
                    .prefix_separator("_")
                    .separator("__"),
            ))
    }
}
