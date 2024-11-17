use config::{Config, Environment, File as ConfigFile, FileFormat};
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
        let config_dir = std::env::var("DIRPIN_CONFIG_DIR").map_or(config_dir(), PathBuf::from);
        let db_path = config_dir.join("server.db");

        let mut config_builder = Config::builder()
            .set_default("host", "127.0.0.1")?
            .set_default("port", 8090)?
            .set_default("db_path", db_path.to_str())?
            .add_source(
                Environment::with_prefix("dirpin")
                    .prefix_separator("_")
                    .separator("__"),
            );

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
}

// TODO: this is duplicate code from the client settings. Possibly merge it inot a dirpin_common or
// find a better way to use it with easier overwrite in the settings.
#[cfg(not(target_os = "windows"))]
pub fn home_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("Failed to find $HOME");
    PathBuf::from(home)
}

#[cfg(target_os = "windows")]
pub fn home_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE").expect("Failed to find %userprofile%");
    PatBuf::from(home)
}

// Get the application configuration directory for the user config
pub fn config_dir() -> PathBuf {
    let config_dir =
        std::env::var("XDG_CONFIG_HOME").map_or_else(|_| home_dir().join(".config"), PathBuf::from);
    config_dir.join("dirpin")
}
