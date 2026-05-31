// Integration tests for fail2ban-kit
// These tests require fail2ban-client and root privileges
// Run with: sudo cargo test --test integration_test -- --ignored

#[cfg(test)]
mod tests {
    use toride_fail2ban::*;

    #[test]
    #[ignore] // requires fail2ban installed
    fn test_ping() {
        let f2b = Fail2Ban::system().expect("fail2ban not available");
        f2b.client().ping().expect("ping failed");
    }

    #[test]
    #[ignore]
    fn test_status() {
        let f2b = Fail2Ban::system().expect("fail2ban not available");
        let status = f2b.client().status().expect("status failed");
        assert!(!status.is_empty());
    }
}
