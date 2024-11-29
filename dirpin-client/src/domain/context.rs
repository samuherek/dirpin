use crate::domain::host::HostId;
use crate::settings::{root_dir, Settings};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Context {
    /// Path from which we build context and workspace. Usually current wokring directory
    pub path: String,
    pub host_id: HostId,
    /// Remote origin
    pub git: Option<String>,
    /// Git directory path
    pub git_path: Option<String>,
}

impl Context {
    const GLOGAL_PATH: &'static str = "/";

    pub fn cwd() -> Self {
        let path = get_current_dir();
        let host_id = Settings::host_id();
        let git_path = get_git_parent_dir(&path);
        let git = get_git_context(&path);

        Self {
            path,
            host_id,
            git,
            git_path,
        }
    }

    pub fn global() -> Self {
        let host_id = Settings::host_id();

        Self {
            path: Self::GLOGAL_PATH.into(),
            host_id,
            git: None,
            git_path: None,
        }
    }

    pub fn workspace_name(&self) -> String {
        let path = if let Some(git_path) = &self.git_path {
            PathBuf::from(git_path.strip_suffix("/").unwrap_or(git_path))
        } else {
            PathBuf::from(self.path.strip_suffix("/").unwrap_or(&self.path))
        };

        path.file_name().unwrap().to_string_lossy().to_string()
    }
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

/// Get the path to the very root of the computer as global path
pub fn get_root_dir() -> String {
    root_dir().to_string_lossy().to_string()
}

/// TODO: Not sure if running this in a command is a good way to go about it.
/// Maybe if you don't have git installed, it would fail and bug people about this?
pub fn get_git_context(path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(&["remote", "get-url", "origin"])
        .current_dir(path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let origin = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Some(origin)
            } else {
                None
            }
        }
        Err(err) => {
            eprintln!("Failed to execute git command: {err}");
            None
        }
    }
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
