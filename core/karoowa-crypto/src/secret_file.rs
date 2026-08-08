//! Writing secret material to disk with restrictive permissions.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

/// Monotonic suffix so two writers in the same process cannot pick the same
/// temp name.
static TEMP_SEQ: AtomicU32 = AtomicU32::new(0);

/// Write secret material (e.g. a private key) to `path` with owner-only
/// permissions (`0600` on Unix).
///
/// Plain `std::fs::write` creates files using the process umask, commonly
/// `0644` — group- and world-readable.
///
/// The write goes to a sibling temp file created `O_EXCL` at `0600` and is then
/// renamed over `path`. That matters for three reasons:
///
/// - The secret is never on disk under a permissive mode, not even briefly.
/// - `path` is replaced atomically, so a failure part-way through cannot leave
///   a truncated or empty key behind. Opening `path` directly with `truncate`
///   would destroy the existing key *before* knowing the write can succeed.
/// - The result is `0600` regardless of what was there before, without needing
///   a `chmod` on the destination — so this also works when the caller can
///   write the directory but does not own the existing file.
///
/// On non-Unix targets there is no mode to set, so the file inherits default
/// ACLs and the caller is responsible for restricting it.
pub fn write_secret_file(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let path = path.as_ref();

    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "secret file path has no file name",
        )
    })?;

    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(
        ".tmp{}.{}",
        std::process::id(),
        TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let temp_path = dir.join(temp_name);

    let result = write_then_rename(&temp_path, path, contents.as_ref());
    if result.is_err() {
        // Never leave a partial secret lying around.
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

fn write_then_rename(temp_path: &Path, path: &Path, contents: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(temp_path)?
    };

    #[cfg(not(unix))]
    let mut file = {
        eprintln!(
            "warning: {} was created without owner-only permissions \
             (unsupported on this platform) — restrict it via filesystem ACLs",
            path.display()
        );
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp_path)?
    };

    file.write_all(contents)?;
    // Flush to disk before the rename, so a crash cannot publish an empty file.
    file.sync_all()?;
    drop(file);

    std::fs::rename(temp_path, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "karoowa-{tag}-{}-{}",
            std::process::id(),
            TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_contents() {
        let dir = temp_dir("secret");
        let path = dir.join("k.key");
        write_secret_file(&path, b"deadbeef").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"deadbeef");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn leaves_no_temp_files_behind() {
        let dir = temp_dir("notemp");
        write_secret_file(dir.join("k.key"), b"x").unwrap();
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "temp file was not cleaned up: {entries:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("perm");
        let path = dir.join("k.key");
        write_secret_file(&path, b"secret").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "key file must not be group/world readable");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn tightens_an_existing_loose_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("reperm");
        let path = dir.join("k.key");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_secret_file(&path, b"new").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(mode, 0o600, "an existing loose key file must be tightened");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn does_not_destroy_the_existing_file_when_the_write_fails() {
        // Regression guard: an earlier version opened the destination with
        // O_TRUNC before it could fail, which emptied a real key.
        //
        // A read-only directory makes the temp-file create fail deterministically.
        // (Don't try to predict the temp name — TEMP_SEQ is shared, so tests
        // running in parallel would race.)
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("nodestroy");
        let path = dir.join("k.key");
        std::fs::write(&path, b"PRECIOUS").unwrap();

        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = write_secret_file(&path, b"new");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            result.is_err(),
            "write should have failed in a ro directory"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"PRECIOUS",
            "a failed write must not damage the existing key"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
