//! Karoowa Enterprise — signed license file parser.
//!
//! This crate ships the real [`SignedLicenseGate`] implementation that
//! replaces the `OssLicenseGate` stub from `karoowa-core::license` at
//! node startup when a valid license file is present.
//!
//! # Format
//!
//! A license file is a JSON document with two top-level fields:
//!
//! ```json
//! {
//!   "payload": {
//!     "edition": "enterprise",
//!     "customer": "Acme Corp",
//!     "issued_at": 1733424000,
//!     "expires_at": 1764960000,
//!     "features": ["rbac", "audit-log", "hsm", "ha"]
//!   },
//!   "signature": "hex-encoded 64-byte ed25519 signature over canonical payload",
//!   "signer_pubkey": "hex-encoded 32-byte ed25519 public key"
//! }
//! ```
//!
//! The signature is computed over the `serde_json::to_vec` of the
//! `payload` object, using the canonical field order below. The
//! verifying public key must match the Karoowa vendor key compiled into
//! the node binary (see [`KAROOWA_VENDOR_PUBKEY`]).
//!
//! # Revocation
//!
//! Offline revocation only in v1.0 — a license can be expired but not
//! remotely disabled. Post-v1.0 a revocation list CRL distribution is
//! planned.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use karoowa_core::{Edition, LicenseGate, LicenseInfo};
use karoowa_crypto::Signature;
use serde::{Deserialize, Serialize};

pub mod error;

pub use error::LicenseError;

/// Well-known vendor public key. In production this is set at build
/// time via a build script that embeds the vendor's real key. For
/// tests, it defaults to all-zeros so test code can pass an explicit
/// vendor pubkey via [`SignedLicenseGate::from_bytes`].
pub const KAROOWA_VENDOR_PUBKEY: [u8; 32] = [0u8; 32];

/// The canonical payload signed by the vendor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LicensePayload {
    /// Edition this license unlocks. Must be `"enterprise"` for any
    /// feature to be gated on.
    pub edition: String,
    /// Customer name (for audit / support). Cosmetic.
    pub customer: String,
    /// Issue timestamp (Unix seconds).
    pub issued_at: u64,
    /// Expiry timestamp (Unix seconds). `None` is not allowed — every
    /// license must have an expiry.
    pub expires_at: u64,
    /// Feature flags this license unlocks. Known flags live in
    /// [`known_features`].
    pub features: Vec<String>,
}

/// A signed license file as loaded from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    pub payload: LicensePayload,
    /// Hex-encoded 64-byte ed25519 signature over
    /// `serde_json::to_vec(&payload)`.
    pub signature: String,
    /// Hex-encoded 32-byte ed25519 public key of the vendor that signed
    /// this license. Checked against the compiled-in vendor pubkey at
    /// verification time to reject licenses signed by unknown keys.
    pub signer_pubkey: String,
}

/// Canonically known enterprise feature flags. License files may
/// reference feature names outside this set — unknown names are
/// rejected by [`SignedLicenseGate::is_feature_enabled`].
pub fn known_features() -> &'static [&'static str] {
    &[
        "rbac",
        "audit-log",
        "hsm",
        "ha",
        "marketplace",
        "multi-tenancy",
    ]
}

/// [`LicenseGate`] implementation backed by a signed license file.
///
/// Construction is fallible: an invalid signature, unknown vendor key,
/// expired license, or malformed JSON all cause [`Self::load`] to
/// return `Err(LicenseError)`. On error the caller should fall back
/// to the OSS gate.
#[derive(Debug, Clone)]
pub struct SignedLicenseGate {
    info: LicenseInfo,
}

impl SignedLicenseGate {
    /// Load, parse, verify, and expiry-check a license file on disk.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, LicenseError> {
        let bytes = fs::read(path.as_ref()).map_err(LicenseError::Io)?;
        Self::from_bytes(&bytes, &KAROOWA_VENDOR_PUBKEY, now_secs())
    }

    /// Parse an in-memory license blob. Primarily for tests; the
    /// public loader is [`Self::load`].
    pub fn from_bytes(
        bytes: &[u8],
        vendor_pubkey: &[u8; 32],
        now: u64,
    ) -> Result<Self, LicenseError> {
        let file: LicenseFile =
            serde_json::from_slice(bytes).map_err(|e| LicenseError::Malformed(e.to_string()))?;

        // 1. Vendor key check.
        let signer_pubkey_bytes = decode_32(&file.signer_pubkey)
            .map_err(|e| LicenseError::Malformed(format!("signer_pubkey: {e}")))?;
        if &signer_pubkey_bytes != vendor_pubkey {
            return Err(LicenseError::UnknownVendor);
        }

        // 2. Signature check over canonical payload bytes.
        let sig_bytes = decode_64(&file.signature)
            .map_err(|e| LicenseError::Malformed(format!("signature: {e}")))?;
        let payload_bytes = serde_json::to_vec(&file.payload)
            .map_err(|e| LicenseError::Malformed(e.to_string()))?;
        let signature = Signature::from_parts(&sig_bytes, &signer_pubkey_bytes)
            .map_err(|_| LicenseError::BadSignature)?;
        signature
            .verify(&payload_bytes)
            .map_err(|_| LicenseError::BadSignature)?;

        // 3. Edition check.
        if file.payload.edition != "enterprise" {
            return Err(LicenseError::WrongEdition(file.payload.edition));
        }

        // 4. Expiry check.
        if now >= file.payload.expires_at {
            return Err(LicenseError::Expired {
                expired_at: file.payload.expires_at,
                now,
            });
        }

        // 5. Normalize features against the known set. Unknown flags
        // are silently dropped — rejecting them would break forward
        // compat when the vendor ships a license with features from a
        // newer node than the customer is running.
        let features: Vec<String> = file
            .payload
            .features
            .iter()
            .filter(|f| known_features().contains(&f.as_str()))
            .cloned()
            .collect();

        Ok(SignedLicenseGate {
            info: LicenseInfo {
                edition: Edition::Enterprise,
                features,
                expires_at: Some(file.payload.expires_at),
            },
        })
    }
}

impl LicenseGate for SignedLicenseGate {
    fn license_info(&self) -> LicenseInfo {
        self.info.clone()
    }

    fn is_feature_enabled(&self, feature: &str) -> bool {
        self.info.features.iter().any(|f| f == feature)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn decode_32(hex: &str) -> Result<[u8; 32], String> {
    let bytes = hex_decode(hex)?;
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn decode_64(hex: &str) -> Result<[u8; 64], String> {
    let bytes = hex_decode(hex)?;
    if bytes.len() != 64 {
        return Err(format!("expected 64 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 64];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_crypto::Keypair;

    fn sign_license(payload: &LicensePayload, keypair: &Keypair) -> LicenseFile {
        let payload_bytes = serde_json::to_vec(payload).unwrap();
        let sig = keypair.sign(&payload_bytes);
        LicenseFile {
            payload: payload.clone(),
            signature: hex_encode(&sig.to_bytes()),
            signer_pubkey: hex_encode(&sig.signer_public_key()),
        }
    }

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn valid_payload() -> LicensePayload {
        LicensePayload {
            edition: "enterprise".into(),
            customer: "Acme".into(),
            issued_at: 1_000_000,
            expires_at: 2_000_000,
            features: vec!["rbac".into(), "audit-log".into()],
        }
    }

    #[test]
    fn valid_license_loads() {
        let keypair = Keypair::generate();
        let vendor_pubkey = keypair.public_key_bytes();
        let file = sign_license(&valid_payload(), &keypair);
        let bytes = serde_json::to_vec(&file).unwrap();

        let gate = SignedLicenseGate::from_bytes(&bytes, &vendor_pubkey, 1_500_000).unwrap();
        let info = gate.license_info();
        assert_eq!(info.edition, Edition::Enterprise);
        assert_eq!(info.features, vec!["rbac", "audit-log"]);
        assert_eq!(info.expires_at, Some(2_000_000));
        assert!(gate.is_feature_enabled("rbac"));
        assert!(gate.is_feature_enabled("audit-log"));
        assert!(!gate.is_feature_enabled("hsm"));
    }

    #[test]
    fn expired_license_rejected() {
        let keypair = Keypair::generate();
        let vendor_pubkey = keypair.public_key_bytes();
        let file = sign_license(&valid_payload(), &keypair);
        let bytes = serde_json::to_vec(&file).unwrap();

        let err = SignedLicenseGate::from_bytes(&bytes, &vendor_pubkey, 3_000_000).unwrap_err();
        assert!(matches!(err, LicenseError::Expired { .. }));
    }

    #[test]
    fn unknown_vendor_rejected() {
        let keypair = Keypair::generate();
        let other_vendor = [9u8; 32];
        let file = sign_license(&valid_payload(), &keypair);
        let bytes = serde_json::to_vec(&file).unwrap();

        let err = SignedLicenseGate::from_bytes(&bytes, &other_vendor, 1_500_000).unwrap_err();
        assert!(matches!(err, LicenseError::UnknownVendor));
    }

    #[test]
    fn tampered_payload_rejected() {
        let keypair = Keypair::generate();
        let vendor_pubkey = keypair.public_key_bytes();
        let mut file = sign_license(&valid_payload(), &keypair);
        // Tamper: grant an extra feature without resigning.
        file.payload.features.push("hsm".into());
        let bytes = serde_json::to_vec(&file).unwrap();

        let err = SignedLicenseGate::from_bytes(&bytes, &vendor_pubkey, 1_500_000).unwrap_err();
        assert!(matches!(err, LicenseError::BadSignature));
    }

    #[test]
    fn oss_edition_rejected() {
        let keypair = Keypair::generate();
        let vendor_pubkey = keypair.public_key_bytes();
        let mut p = valid_payload();
        p.edition = "oss".into();
        let file = sign_license(&p, &keypair);
        let bytes = serde_json::to_vec(&file).unwrap();

        let err = SignedLicenseGate::from_bytes(&bytes, &vendor_pubkey, 1_500_000).unwrap_err();
        assert!(matches!(err, LicenseError::WrongEdition(_)));
    }

    #[test]
    fn unknown_feature_flags_silently_dropped() {
        let keypair = Keypair::generate();
        let vendor_pubkey = keypair.public_key_bytes();
        let mut p = valid_payload();
        p.features = vec!["rbac".into(), "time-travel".into(), "hsm".into()];
        let file = sign_license(&p, &keypair);
        let bytes = serde_json::to_vec(&file).unwrap();

        let gate = SignedLicenseGate::from_bytes(&bytes, &vendor_pubkey, 1_500_000).unwrap();
        assert!(gate.is_feature_enabled("rbac"));
        assert!(gate.is_feature_enabled("hsm"));
        assert!(!gate.is_feature_enabled("time-travel"));
    }

    #[test]
    fn malformed_json_rejected() {
        let err = SignedLicenseGate::from_bytes(b"not json", &[0u8; 32], 1_500_000).unwrap_err();
        assert!(matches!(err, LicenseError::Malformed(_)));
    }
}
