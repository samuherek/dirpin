use dirpin_client::settings::Settings;

use crate::VERSION;

pub fn run(settings: &Settings) {
    let config = dirpin_common::utils::config_dir();
    let mut config_file = config.clone();
    config_file.push("config.toml");

    let paths = format!(
        "PATHS:\nconfig_path: {:?}\ndb_path: {:?}\nkey_path: {:?}\nsession_path: {:?}",
        config_file, settings.db_path, settings.key_path, settings.session_path
    );

    println!("{paths}\n\nVersion: {VERSION}");
}
