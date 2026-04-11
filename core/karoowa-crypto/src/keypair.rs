//! Ed25519 keypair and signature types.
//!
//! Wraps `ed25519-dalek` with Karoowa-specific ergonomics: address derivation,
//! hex encoding, and serde support. Keys are always generated from OS entropy
//! via [`OsRng`](rand_core::OsRng) — no hand-rolled randomness.

use crate::address::Address;
use ed25519_dalek::{Signer, Verifier};
use rand_core::OsRng;
use std::fmt;

/// An Ed25519 signing keypair (private + public key).
///
/// Generate with [`Keypair::generate`]. The private key never leaves this
/// struct — use [`Keypair::sign`] to produce signatures.
pub struct Keypair {
    inner: ed25519_dalek::SigningKey,
}

impl Keypair {
    /// Generate a new keypair from OS entropy.
    pub fn generate() -> Self {
        Keypair {
            inner: ed25519_dalek::SigningKey::generate(&mut OsRng),
        }
    }

    /// Reconstruct a keypair from a 32-byte seed (private key bytes).
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Keypair {
            inner: ed25519_dalek::SigningKey::from_bytes(seed),
        }
    }

    /// The public key bytes (32 bytes).
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.inner.verifying_key().to_bytes()
    }

    /// The Karoowa address derived from this keypair's public key.
    pub fn address(&self) -> Address {
        Address::from_public_key(&self.public_key_bytes())
    }

    /// The private key bytes (32-byte seed). Handle with care.
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// The private key as a hex string with `0x` prefix.
    pub fn private_key_hex(&self) -> String {
        format!("0x{}", hex::encode(self.private_key_bytes()))
    }

    /// Sign a message, returning a [`Signature`].
    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.inner.sign(message);
        Signature {
            inner: sig,
            public_key: self.inner.verifying_key(),
        }
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never print the private key in debug output.
        write!(f, "Keypair(address={})", self.address())
    }
}

/// An Ed25519 signature with the signer's public key attached.
///
/// This is the output of [`Keypair::sign`]. Verify with [`Signature::verify`].
#[derive(Clone)]
pub struct Signature {
    inner: ed25519_dalek::Signature,
    public_key: ed25519_dalek::VerifyingKey,
}

impl Signature {
    /// The raw signature bytes (64 bytes).
    pub fn to_bytes(&self) -> [u8; 64] {
        self.inner.to_bytes()
    }

    /// The signer's public key bytes (32 bytes).
    pub fn signer_public_key(&self) -> [u8; 32] {
        self.public_key.to_bytes()
    }

    /// The signer's Karoowa address.
    pub fn signer_address(&self) -> Address {
        Address::from_public_key(&self.signer_public_key())
    }

    /// Verify this signature against a message.
    ///
    /// Returns `Ok(())` if the signature is valid, `Err` otherwise.
    pub fn verify(&self, message: &[u8]) -> Result<(), SignatureError> {
        self.public_key
            .verify(message, &self.inner)
            .map_err(|_| SignatureError::Invalid)
    }

    /// Reconstruct a `Signature` from raw bytes + public key bytes.
    ///
    /// Used when deserializing signatures from the network or storage.
    pub fn from_parts(
        sig_bytes: &[u8; 64],
        pubkey_bytes: &[u8; 32],
    ) -> Result<Self, SignatureError> {
        let inner = ed25519_dalek::Signature::from_bytes(sig_bytes);
        let public_key = ed25519_dalek::VerifyingKey::from_bytes(pubkey_bytes)
            .map_err(|_| SignatureError::InvalidPublicKey)?;
        Ok(Signature { inner, public_key })
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Signature(signer={}, sig=0x{}...)",
            self.signer_address(),
            hex::encode(&self.to_bytes()[..8])
        )
    }
}

/// Errors related to signatures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SignatureError {
    /// The signature failed verification against the message and public key.
    #[error("signature verification failed")]
    Invalid,
    /// The public key bytes were not a valid Ed25519 public key.
    #[error("invalid public key")]
    InvalidPublicKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_unique_keypairs() {
        let kp1 = Keypair::generate();
        let kp2 = Keypair::generate();
        assert_ne!(kp1.public_key_bytes(), kp2.public_key_bytes());
    }

    #[test]
    fn from_seed_is_deterministic() {
        let seed = [42u8; 32];
        let kp1 = Keypair::from_seed(&seed);
        let kp2 = Keypair::from_seed(&seed);
        assert_eq!(kp1.public_key_bytes(), kp2.public_key_bytes());
        assert_eq!(kp1.address(), kp2.address());
    }

    #[test]
    fn address_is_20_bytes() {
        let kp = Keypair::generate();
        assert_eq!(kp.address().as_bytes().len(), 20);
    }

    #[test]
    fn sign_and_verify_succeeds() {
        let kp = Keypair::generate();
        let msg = b"hello karoowa";
        let sig = kp.sign(msg);
        assert!(sig.verify(msg).is_ok());
    }

    #[test]
    fn verify_wrong_message_fails() {
        let kp = Keypair::generate();
        let sig = kp.sign(b"correct message");
        assert!(sig.verify(b"wrong message").is_err());
    }

    #[test]
    fn verify_wrong_signer_fails() {
        let kp1 = Keypair::generate();
        let kp2 = Keypair::generate();
        let msg = b"test";
        let sig = kp1.sign(msg);

        // Reconstruct with kp2's public key — should fail
        let bad_sig = Signature::from_parts(&sig.to_bytes(), &kp2.public_key_bytes()).unwrap();
        assert!(bad_sig.verify(msg).is_err());
    }

    #[test]
    fn signer_address_matches_keypair_address() {
        let kp = Keypair::generate();
        let sig = kp.sign(b"test");
        assert_eq!(sig.signer_address(), kp.address());
    }

    #[test]
    fn signature_from_parts_roundtrip() {
        let kp = Keypair::generate();
        let msg = b"roundtrip";
        let sig = kp.sign(msg);

        let reconstructed =
            Signature::from_parts(&sig.to_bytes(), &sig.signer_public_key()).unwrap();
        assert!(reconstructed.verify(msg).is_ok());
        assert_eq!(reconstructed.signer_address(), kp.address());
    }

    #[test]
    fn private_key_hex_roundtrip() {
        let kp = Keypair::generate();
        let hex_str = kp.private_key_hex();
        assert!(hex_str.starts_with("0x"));
        assert_eq!(hex_str.len(), 66); // "0x" + 64 hex chars
    }

    #[test]
    fn debug_does_not_leak_private_key() {
        let kp = Keypair::generate();
        let debug = format!("{kp:?}");
        assert!(!debug.contains(&hex::encode(kp.private_key_bytes())));
        assert!(debug.contains("Keypair(address="));
    }
}
