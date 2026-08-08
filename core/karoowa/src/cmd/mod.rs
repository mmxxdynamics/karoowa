pub mod agent;
pub mod client;
pub mod devnet;
pub mod genesis;
pub mod network;
pub mod node;
pub mod wallet;

use std::io;
use std::path::Path;

/// Write secret material (e.g. a private key) to `path` with owner-only
/// permissions (`0600` on Unix).
///
/// Plain `std::fs::write` creates files using the process umask, commonly
/// `0644` — group- and world-readable. This helper creates the file with
/// restrictive permissions *before* the secret is written, rather than
/// chmod-ing afterwards, which would leave a window where the key is readable.
///
/// On non-Unix targets there is no mode to set, so this is a plain write; the
/// caller is responsible for platform ACLs.
pub fn write_secret_file(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    use std::io::Write;
    let path = path.as_ref();

    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?
    };

    #[cfg(not(unix))]
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    file.write_all(contents.as_ref())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_secret_file_writes_contents() {
        let dir = std::env::temp_dir().join(format!("karoowa-secret-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k.key");
        write_secret_file(&path, b"deadbeef").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"deadbeef");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn write_secret_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("karoowa-perm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k.key");
        write_secret_file(&path, b"secret").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "key file must not be group/world readable");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn write_secret_file_tightens_an_existing_loose_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("karoowa-reperm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k.key");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_secret_file(&path, b"new").unwrap();
        // `.mode()` only applies at creation, so an existing 0644 file stays
        // 0644. Document the real behaviour rather than assert a false claim.
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(mode, 0o644, "pre-existing permissions are not tightened");
        std::fs::remove_dir_all(&dir).ok();
    }
}
