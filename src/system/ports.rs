pub fn port_in_use(port: u16) -> bool {
    std::process::Command::new("ss")
        .args(["-tlnp"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .contains(&format!(":{}", port))
        })
        .unwrap_or(false)
}
