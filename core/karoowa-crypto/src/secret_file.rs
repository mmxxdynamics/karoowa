//! Writing secret material to disk with restrictive permissions.

use std::io;
use std::path::Path;

/// Write secret material (e.g. a private key) to `path` with owner-only
/// permissions (`0600` on Unix).
///
/// Plain `std::fs::write` creates files using the process umask, commonly
/// `0644` — group- and world-readable. This helper creates the file with
/// restrictive permissions *before* the secret is written, so there is no
/// window in which a fresh key is readable.
///
/// `OpenOptions::mode()` only applies at creation, so an existing loose file
/// would keep its old mode. The explicit `set_permissions` on the open handle
/// (an `fchmod`, not a path-based `chmod`) covers that case too, without a
/// symlink race.
///
/// On non-Unix targets there is no mode to set, so this is a plain write and
/// the caller is responsible for platform ACLs.
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
    let mut file = {
        // No mode to set here; the file inherits default ACLs. Say so rather
        // than let the caller assume the same protection as Unix.
        eprintln!(
            "warning: {} was created without owner-only permissions \
             (unsupported on this platform) — restrict it via filesystem ACLs",
            path.display()
        );
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?
    };

    // Covers the pre-existing-file case, where `.mode()` above is a no-op.
    // Done on the handle so it cannot be redirected between open and chmod.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

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
        // Regression guard: `.mode()` alone does not cover this path.
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("karoowa-reperm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k.key");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_secret_file(&path, b"new").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(mode, 0o600, "an existing loose key file must be tightened");
        std::fs::remove_dir_all(&dir).ok();
    }
}
