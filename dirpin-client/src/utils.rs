pub(crate) fn get_hostname() -> String {
    whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string())
}

pub(crate) fn get_username() -> String {
    whoami::username()
}

pub(crate) fn get_host_user() -> String {
    format!("{}:{}", get_hostname(), get_username())
}
