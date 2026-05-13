pub fn apt_update() -> std::process::Command {
    let mut cmd = std::process::Command::new("flock");
    cmd.args(["-w", "600", "/var/lib/dpkg/lock-frontend", "apt-get", "update"]);
    cmd
}

pub fn apt_install(packages: &[&str]) -> std::process::Command {
    let mut cmd = std::process::Command::new("flock");
    cmd.args(["-w", "600", "/var/lib/dpkg/lock-frontend", "apt-get", "install", "-y"]);
    cmd.args(packages);
    cmd
}
