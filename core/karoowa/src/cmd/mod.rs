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
/// Plain `std::fs::write` creates files with the process umask (commonly
/// `0644`), leaving private keys group/world-readable. This helper creates the
/// file with restrictive permissions *before* the secret is written.
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
