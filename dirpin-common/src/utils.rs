use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use getrandom::getrandom;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

// TODO: this is duplicate code from the server settings. Possibly merge it inot a dirpin_common or
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

/// Get the application data directory for internal data
pub fn data_dir() -> PathBuf {
    let data_dir = std::env::var("XDG_DATA_HOME")
        .map_or_else(|_| home_dir().join(".local").join("share"), PathBuf::from);
    data_dir.join("dirpin")
}

pub fn read_pipe_value() -> Result<Option<String>, io::Error> {
    let mut stdin = io::stdin();
    if stdin.is_terminal() {
        Ok(None)
    } else {
        let mut buf = String::new();
        stdin.read_to_string(&mut buf)?;
        let value = if buf.is_empty() { None } else { Some(buf) };
        Ok(value)
    }
}

/// Generate N random bytes, using a cryptographically secure source
pub fn crypto_random_bytes<const N: usize>() -> [u8; N] {
    // rand say they are in principle safe for crypto purposes, but that it is perhaps a better
    // idea to use getrandom for things such as passwords.
    let mut ret = [0u8; N];

    getrandom(&mut ret).expect("Failed to generate random bytes!");

    ret
}

/// Generate N random bytes using a cryptographically secure source, return encoded as a string
pub fn crypto_random_string<const N: usize>() -> String {
    let bytes = crypto_random_bytes::<N>();

    // We only use this to create a random string, and won't be reversing it to find the original
    // data - no padding is OK there. It may be in URLs.
    BASE64_URL_SAFE_NO_PAD.encode(bytes)
}
