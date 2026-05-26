//! Fixed-size hash type and hashing functions.
//!
//! [`struct@Hash`] is a 32-byte value used everywhere in Karoowa: block hashes,
//! transaction hashes, Merkle nodes, state roots. Two hash functions are
//! provided:
//!
//! - [`sha3_256`]: primary hash used for consensus-critical operations.
//! - [`blake3_hash`]: fast-path hash used where speed matters more than
//!   compatibility (e.g. local caching, content addressing).

use sha3::{Digest, Sha3_256};
use std::fmt;

/// A 32-byte hash value.
///
/// Serializes as a hex string (e.g. `"0xabcd..."`) for JSON readability.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Hash([u8; 32]);

impl serde::Serialize for Hash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> serde::Deserialize<'de> for Hash {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Hash::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

impl Hash {
    /// The zero hash (all bytes 0x00).
    pub const ZERO: Hash = Hash([0u8; 32]);

    /// Create a `Hash` from raw bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }

    /// Return the underlying bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Return the underlying bytes as a slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Create a `Hash` from a hex string (with or without `0x` prefix).
    pub fn from_hex(s: &str) -> Result<Self, HashError> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|_| HashError::InvalidHex)?;
        if bytes.len() != 32 {
            return Err(HashError::InvalidLength {
                expected: 32,
                got: bytes.len(),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Hash(arr))
    }

    /// Encode as a hex string with `0x` prefix.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self.to_hex())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl std::str::FromStr for Hash {
    type Err = HashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Hash::from_hex(s)
    }
}

/// Errors when parsing a [`struct@Hash`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum HashError {
    /// The hex string was not valid hex.
    #[error("invalid hex encoding")]
    InvalidHex,
    /// The decoded bytes had the wrong length.
    #[error("invalid hash length: expected {expected}, got {got}")]
    InvalidLength { expected: usize, got: usize },
}

// ---------------------------------------------------------------------------
// Hashing functions
// ---------------------------------------------------------------------------

/// Compute the SHA3-256 hash of the given data.
///
/// This is the **primary hash function** used for consensus-critical
/// operations throughout Karoowa (block hashes, tx hashes, Merkle nodes,
/// address derivation).
pub fn sha3_256(data: &[u8]) -> Hash {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let result = hasher.finalize();
    Hash(result.into())
}

/// Compute the BLAKE3 hash of the given data, truncated to 32 bytes.
///
/// This is the **fast-path hash** used for non-consensus-critical operations
/// where speed matters more than SHA3 compatibility (e.g. local caching,
/// content-addressed storage).
pub fn blake3_hash(data: &[u8]) -> Hash {
    let h = blake3::hash(data);
    Hash(*h.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha3_256_known_vector() {
        // SHA3-256("") = a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a
        let hash = sha3_256(b"");
        assert_eq!(
            hash.to_hex(),
            "0xa7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
        );
    }

    #[test]
    fn sha3_256_hello() {
        let hash = sha3_256(b"hello");
        // SHA3-256("hello") = 3338be694f50c5f338814986cdf0686453a888b84f424d792af4b9202398f392
        assert_eq!(
            hash.to_hex(),
            "0x3338be694f50c5f338814986cdf0686453a888b84f424d792af4b9202398f392"
        );
    }

    #[test]
    fn blake3_produces_32_bytes() {
        let hash = blake3_hash(b"test data");
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn blake3_deterministic() {
        let a = blake3_hash(b"same input");
        let b = blake3_hash(b"same input");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_hex_roundtrip() {
        let original = sha3_256(b"roundtrip test");
        let hex_str = original.to_hex();
        let parsed = Hash::from_hex(&hex_str).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn hash_from_hex_no_prefix() {
        let hash = sha3_256(b"");
        let hex_no_prefix = "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a";
        let parsed = Hash::from_hex(hex_no_prefix).unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn hash_from_hex_invalid() {
        assert!(Hash::from_hex("not hex").is_err());
        assert!(Hash::from_hex("0xabcd").is_err()); // too short
    }

    #[test]
    fn hash_display_and_debug() {
        let hash = Hash::ZERO;
        let display = format!("{hash}");
        let debug = format!("{hash:?}");
        assert!(display.starts_with("0x"));
        assert!(debug.starts_with("Hash(0x"));
    }

    #[test]
    fn hash_from_str() {
        let hex = "0xa7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a";
        let hash: Hash = hex.parse().unwrap();
        assert_eq!(hash, sha3_256(b""));
    }

    #[test]
    fn hash_serde_roundtrip() {
        let original = sha3_256(b"serde test");
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Hash = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn hash_zero() {
        assert_eq!(Hash::ZERO.as_bytes(), &[0u8; 32]);
    }
}
