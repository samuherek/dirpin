use std::path::PathBuf;

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

// Get the path to the current directory based on the env variable.
pub fn get_current_dir() -> String {
    std::env::current_dir()
        .expect("Failed to load currnet dir")
        .to_string_lossy()
        .to_string()
}

// Checks if the current directory has a ".git" directory in it.
pub fn has_git_dir(path: &str) -> bool {
    let mut gitdir = PathBuf::from(path);
    gitdir.push(".git");
    gitdir.exists()
}

/// Get the path to the parent directory that contains a ".git" 
pub fn get_git_parent_dir(path: &str) -> Option<String> {
    let mut path = PathBuf::from(path);
    while path.parent().is_some() && !has_git_dir(path.to_str().unwrap()) {
        path.pop();
    }

    if path.parent().is_some() {
        return Some(path.to_string_lossy().to_string());
    }

    None
}
