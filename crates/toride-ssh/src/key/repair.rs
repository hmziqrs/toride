//! Repair missing public key files from private keys.

use std::path::Path;

use crate::{Error, Result};

/// Derive and write the `.pub` file for a private key.
pub async fn repair_public_key(private_key_path: &Path) -> Result<()> {
    // Validate the path before spawning the blocking task.
    if !private_key_path.exists() {
        return Err(Error::KeyNotFound(private_key_path.display().to_string()));
    }
    if !private_key_path.is_file() {
        return Err(Error::KeyParseFailed(format!(
            "{} is not a regular file",
            private_key_path.display()
        )));
    }

    let private_path = private_key_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        // Read and parse the private key
        let private_key_data = std::fs::read_to_string(&private_path).map_err(|e| {
            Error::KeyParseFailed(format!(
                "failed to read private key {}: {e}",
                private_path.display()
            ))
        })?;

        let private_key = ssh_key::PrivateKey::from_openssh(&private_key_data).map_err(|e| {
            Error::KeyParseFailed(format!(
                "failed to parse private key {}: {e}",
                private_path.display()
            ))
        })?;

        // Derive the public key path
        let public_path = private_path.with_extension("pub");

        // Get the public key and write it
        let public_key = private_key.public_key();
        public_key
            .write_openssh_file(&public_path)
            .map_err(|e| {
                Error::CommandFailed(format!(
                    "failed to write public key {}: {e}",
                    public_path.display()
                ))
            })?;

        // Set permissions to 0o644 on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&public_path, std::fs::Permissions::from_mode(0o644))
                .map_err(|e| {
                    Error::Io(e)
                })?;
        }

        Ok(())
    })
    .await
    .map_err(|e| Error::TaskFailed(format!("repair_public_key task failed: {e}")))?
}
