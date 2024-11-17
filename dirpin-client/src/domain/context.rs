use crate::domain::host::HostId;
use crate::domain::workspace::WorkspaceId;
use crate::settings::{root_dir, Settings};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Context {
    pub path: String,
    pub host_id: HostId,
    pub workspace_id: Option<WorkspaceId>,
    pub git: Option<String>,
}

impl Context {
    pub fn cwd(_settings: &Settings) -> Self {
        let path = get_current_dir();
        let host_id = Settings::host_id();
        // TODO: git directory
        // TODO: workspace_id. Where does it come from? From the db?
        Self {
            path,
            host_id,
            workspace_id: None,
            git: None,
        }
    }

    pub fn global(_settings: &Settings) -> Self {
        let host_id = Settings::host_id();
        Self {
            path: "/".into(),
            host_id,
            workspace_id: None,
            git: None,
        }
    }
}

/// We assume that the global context is the root of the computer
/// and we assume there is no "git repo" in the root of the computer
/// TODO: Please check the "get_root_dir" impl for comment about the
/// widnows root dir.
// pub fn global_context() -> Context {
//     let hostname = get_host_user();
//     let paht = get_root_dir();
//     Context {
//         path: "".into(),
//         host_id: HostId("".into()),
//         workspace_id: None,
//         git: None,
//     }
// }

/// Get the current entry context basd on the current directory path
// pub fn current_context() -> Context {
//     let hostname = get_host_user();
//     let cwd = get_current_dir();
//     let cgd = get_git_parent_dir(&cwd);
//     Context { cwd, hostname, cgd }
// }

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
