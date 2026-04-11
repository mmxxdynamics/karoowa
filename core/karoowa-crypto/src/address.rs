//! Karoowa account address.
//!
//! An [`Address`] is the last 20 bytes of `SHA3-256(public_key)`. It uniquely
//! identifies an account (user or contract) on any Karoowa chain.

use crate::hash::{sha3_256, HashError};
use std::fmt;

/// A 20-byte account address derived from a public key.
///
/// Serializes as a hex string (e.g. `"0x1234..."`) for JSON compatibility
/// (including use as JSON map keys).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Address([u8; 20]);

impl serde::Serialize for Address {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> serde::Deserialize<'de> for Address {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Address::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

impl Address {
    /// The zero address (all bytes 0x00).
    pub const ZERO: Address = Address([0u8; 20]);

    /// Create an `Address` from raw bytes.
    pub const fn from_bytes(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }

    /// Return the underlying bytes.
    pub const fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Derive an address from a public key (last 20 bytes of SHA3-256).
    pub fn from_public_key(public_key: &[u8]) -> Self {
        let hash = sha3_256(public_key);
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash.as_bytes()[12..32]);
        Address(addr)
    }

    /// Parse an address from a hex string (with or without `0x` prefix).
    pub fn from_hex(s: &str) -> Result<Self, AddressError> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|_| AddressError::InvalidHex)?;
        if bytes.len() != 20 {
            return Err(AddressError::InvalidLength {
                expected: 20,
                got: bytes.len(),
            });
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Address(arr))
    }

    /// Encode as a hex string with `0x` prefix.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

impl From<[u8; 20]> for Address {
    fn from(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self.to_hex())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl std::str::FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Address::from_hex(s)
    }
}

/// Errors when parsing an [`Address`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum AddressError {
    /// The hex string was not valid hex.
    #[error("invalid hex encoding")]
    InvalidHex,
    /// The decoded bytes had the wrong length.
    #[error("invalid address length: expected {expected}, got {got}")]
    InvalidLength { expected: usize, got: usize },
    /// Wraps a [`HashError`] for convenience.
    #[error(transparent)]
    Hash(#[from] HashError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_from_public_key_is_20_bytes() {
        let fake_pubkey = [42u8; 32];
        let addr = Address::from_public_key(&fake_pubkey);
        assert_eq!(addr.as_bytes().len(), 20);
    }

    #[test]
    fn address_from_public_key_deterministic() {
        let pubkey = [7u8; 32];
        let a = Address::from_public_key(&pubkey);
        let b = Address::from_public_key(&pubkey);
        assert_eq!(a, b);
    }

    #[test]
    fn address_from_public_key_uses_last_20_of_sha3() {
        let pubkey = [99u8; 32];
        let hash = sha3_256(&pubkey);
        let addr = Address::from_public_key(&pubkey);
        assert_eq!(addr.as_bytes(), &hash.as_bytes()[12..32]);
    }

    #[test]
    fn address_hex_roundtrip() {
        let addr = Address::from_public_key(&[1u8; 32]);
        let hex_str = addr.to_hex();
        let parsed = Address::from_hex(&hex_str).unwrap();
        assert_eq!(addr, parsed);
    }

    #[test]
    fn address_from_hex_no_prefix() {
        let addr = Address::from_public_key(&[1u8; 32]);
        let hex_no_prefix = hex::encode(addr.as_bytes());
        let parsed = Address::from_hex(&hex_no_prefix).unwrap();
        assert_eq!(addr, parsed);
    }

    #[test]
    fn address_from_hex_invalid() {
        assert!(Address::from_hex("not hex").is_err());
        assert!(Address::from_hex("0xabcd").is_err()); // too short
    }

    #[test]
    fn address_display() {
        let addr = Address::ZERO;
        let s = format!("{addr}");
        assert_eq!(s, "0x0000000000000000000000000000000000000000");
    }

    #[test]
    fn address_serde_roundtrip() {
        let addr = Address::from_public_key(&[55u8; 32]);
        let json = serde_json::to_string(&addr).unwrap();
        let deserialized: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, deserialized);
    }
}
