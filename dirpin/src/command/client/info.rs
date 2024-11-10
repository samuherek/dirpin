use dirpin_client::settings::Settings;
use std::path::PathBuf;
use time::{Duration, OffsetDateTime};

use crate::VERSION;

fn naive_time_ago(timestamp: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let duration = now - timestamp;

    if duration < Duration::hours(1) {
        format!("{} minutes ago", duration.whole_minutes())
    } else if duration < Duration::days(1) {
        format!("{} hours ago", duration.whole_hours())
    } else {
        format!("{} days ago", duration.whole_days())
    }
}

pub fn run(settings: &Settings) {
    let env_config_dir = std::env::var("DIRPIN_CONFIG_DIR");

    let config_dir = if let Ok(config_dir) = &env_config_dir {
        PathBuf::from(config_dir)
    } else {
        dirpin_common::utils::config_dir()
    };

    let mut config_file = config_dir.clone();
    config_file.push("config.toml");

    let vars = format!(
        "VARS:\nDIRPIN_CONFIG_DIR = {:?}",
        env_config_dir.unwrap_or("None".into())
    );
    println!("{vars}\n");

    let mut paths = String::from("PATHS:\n");
    paths.push_str(&format!("config_path: {config_file:?}\n"));
    paths.push_str(&format!("db_path: {:?}\n", settings.db_path));
    paths.push_str(&format!("key_path: {:?}\n", settings.key_path));
    paths.push_str(&format!("session_path: {:?}", settings.session_path));
    println!("{paths}\n");

    println!("ACCOUNT: ");
    println!("Host_id: {}", Settings::host_id());
    println!("Hostname: {}", "TODO");
    println!(
        "Auth: {}",
        settings.session().unwrap_or("Unauthenticated".into())
    );
    println!(
        "Last sync: {}",
        Settings::last_sync()
            .map(|x| naive_time_ago(x))
            .unwrap_or("Unavailable".to_string())
    );

    println!("");
    println!("Version: {VERSION}");
}
