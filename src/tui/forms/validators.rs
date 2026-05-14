pub fn username(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("Username is required".into());
    }
    if input == "root" {
        return Err("Cannot use 'root'".into());
    }
    if !input.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
        return Err("Must start with a letter or underscore".into());
    }
    if !input.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("Only alphanumeric, underscore, and hyphen allowed".into());
    }
    if input.len() > 32 {
        return Err("Username too long (max 32 chars)".into());
    }
    Ok(())
}

pub fn ssh_public_key(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("SSH public key is required".into());
    }
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid SSH key format (expected: type base64 [comment])".into());
    }
    let key_type = parts[0];
    if !matches!(key_type, "ssh-rsa" | "ssh-ed25519" | "ssh-ecdsa" | "ecdsa-sha2-nistp256" | "ecdsa-sha2-nistp384" | "ecdsa-sha2-nistp521" | "ssh-dss") {
        return Err(format!("Unsupported key type: {}", key_type));
    }
    if input.contains("PRIVATE KEY") {
        return Err("This appears to be a private key. Please provide a public key.".into());
    }
    Ok(())
}

pub fn swap_size(input: &str) -> Result<(), String> {
    if input.is_empty() || input == "0" {
        return Ok(());
    }
    let input = input.trim().to_uppercase();
    let (num_part, unit) = if input.ends_with('G') {
        (&input[..input.len()-1], 'G')
    } else if input.ends_with('M') {
        (&input[..input.len()-1], 'M')
    } else if input.ends_with('K') {
        (&input[..input.len()-1], 'K')
    } else {
        return Err("Must end with K, M, or G (e.g. 2G, 512M)".into());
    };
    let num: u64 = num_part.parse().map_err(|_| "Invalid number".to_string())?;
    if num == 0 {
        return Ok(());
    }
    match unit {
        'G' if num > 64 => return Err("Swap size too large (max 64G)".into()),
        'M' if num > 64000 => return Err("Swap size too large (max 64000M)".into()),
        'K' if num > 64000000 => return Err("Swap size too large".into()),
        _ => {}
    }
    Ok(())
}

pub fn port(input: &str) -> Result<(), String> {
    let port: u16 = input.parse().map_err(|_| "Must be a number".to_string())?;
    if port == 0 {
        return Err("Port 0 is not valid".into());
    }
    if port < 1024 && port != 22 {
        // warning only, not error
    }
    Ok(())
}

pub fn hostname(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("Hostname is required".into());
    }
    if input.len() > 63 {
        return Err("Hostname too long (max 63 chars)".into());
    }
    if input.starts_with('-') || input.ends_with('-') {
        return Err("Hostname cannot start or end with hyphen".into());
    }
    if !input.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("Only alphanumeric and hyphen allowed".into());
    }
    Ok(())
}
