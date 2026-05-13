pub fn current_user() -> String {
    std::env::var("USER").unwrap_or_else(|_| "unknown".into())
}

pub fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

pub fn user_exists(name: &str) -> bool {
    nix::unistd::User::from_name(name).ok().flatten().is_some()
}
