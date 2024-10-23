use dirpin_client::settings::Settings;
use std::path::PathBuf;

use crate::VERSION;

pub fn run(settings: &Settings) {
    let env_config_dir = std::env::var("DIRPIN_CONFIG_DIR");

    let config_dir = if let Ok(config_dir) = &env_config_dir {
        PathBuf::from(config_dir)
    } else {
        dirpin_common::utils::config_dir()
    };

    let mut config_file = config_dir.clone();
    config_file.push("config.toml");

    let paths = format!(
        "PATHS:\nconfig_path: {:?}\ndb_path: {:?}\nkey_path: {:?}\nsession_path: {:?}",
        config_file, settings.db_path, settings.key_path, settings.session_path
    );
    let vars = format!(
        "VARS:\nDIRPIN_CONFIG_DIR = {:?}",
        env_config_dir.unwrap_or("None".into())
    );

    println!("{vars}\n\n{paths}\n\nVersion: {VERSION}");
}
