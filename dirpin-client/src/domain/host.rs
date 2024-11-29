use std::str::FromStr;

/// Hostname is generated from the whoami command that gets the hostname and the username of the
/// machine. It will then convert it to the format of "{username}@{hostname}";
///
/// This is not necessarily a unique identifier as nothing stops the user from having two machines
/// being called the same and have the same user.
///
/// TODO: Not sure if we need to have a unique identifier or not. I can see that having a readable
/// identifier might be more usefull for filtering and I am not sure if we'd allow some specialised
/// unique entries for just that host with the unique id. In theory, unless you have two machines
/// with the same name and user, this issue would not arise anyway. And since this is a developer
/// tool and not a corportaion tool, I think it's safe to assume that the id not being unique is
/// highly unlikely at this point.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HostId(String);

impl HostId {
    pub fn custom(username: String, host: String) -> Self {
        Self(format_host_user(username, host))
    }

    /// Format "{username}@{hostname}"
    pub fn get_host_id() -> Self {
        Self(format_host_user(get_username(), get_hostname()))
    }
}

impl FromStr for HostId {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('@').collect::<Vec<_>>();

        if parts.len() != 2 {
            return Err("Input should have exactly two parts separated by '@'");
        }
        // TODO: do we only wan to allow asci?
        if !parts[0].chars().all(char::is_alphanumeric) {
            return Err("First split needs to contain only alphanumerci values");
        }
        if !parts[1].chars().all(char::is_alphanumeric) {
            return Err("Second split needs to contain only alphanumerci values");
        }

        Ok(Self(s.into()))
    }
}

impl AsRef<str> for HostId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn get_hostname() -> String {
    whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string())
}

fn get_username() -> String {
    whoami::username()
}

/// Format "{username}@{hostname}"
fn format_host_user(user: String, host: String) -> String {
    format!("{}@{}", user, host)
}
