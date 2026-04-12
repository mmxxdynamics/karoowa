//! Karoowa Enterprise — certified agent marketplace.
//!
//! # Why
//!
//! Karoowa's agent runtime (`core/karoowa-agents`) happily loads any
//! agent binary an operator points it at. That's fine for dev and for
//! customer-written agents, but enterprise customers want a curated
//! set of **certified** agents: third-party agents whose bytecode has
//! been reviewed by Karoowa and signed with a vendor attestation key.
//!
//! This crate implements the registry and the attestation verifier.
//! It ships the `CertifiedAgent` wire format, the `Registry` type
//! that indexes agents by id and content hash, and
//! [`AttestationSigner`] / [`AttestationVerifier`] helpers wrapping
//! ed25519.
//!
//! # Out of scope (post-v1.0)
//!
//! - The marketplace website (listing UI, payments, download CDN).
//!   That's a Phase 6.4 ops deliverable, not code.
//! - Attestation key rotation and revocation. v1.0 ships a single
//!   compiled-in verifier key; rotation lands in v1.1 alongside the
//!   license CRL work.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use karoowa_crypto::{sha3_256, Hash, Keypair, Signature};
use serde::{Deserialize, Serialize};

pub mod error;

pub use error::MarketplaceError;

/// Stable identifier for a certified agent. By convention the id is
/// a reverse-DNS string: `com.karoowa.governance-copilot`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        AgentId(id.into())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What this agent is specialized for. Informational — enforcement
/// is the integrator's responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCategory {
    Governance,
    Treasury,
    Security,
    Observability,
    Optimizer,
    Other,
}

/// Canonical fields signed by the Karoowa attestation key. Serialize
/// the payload, sign the bytes, ship the pair. Any field changing
/// without a re-signature invalidates the attestation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentAttestationPayload {
    pub id: AgentId,
    pub version: String,
    pub category: AgentCategory,
    /// SHA3-256 of the agent's binary or WASM bytecode.
    pub content_hash: Hash,
    /// Unix seconds when the attestation was issued.
    pub issued_at: u64,
    /// Unix seconds when the attestation expires.
    pub expires_at: u64,
    /// Human-readable vendor name (for display).
    pub vendor: String,
}

/// A certified agent: attestation payload + vendor signature +
/// vendor pubkey. Loaded from disk and verified at registration time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedAgent {
    pub payload: AgentAttestationPayload,
    /// Hex-encoded 64-byte ed25519 signature over
    /// `serde_json::to_vec(&payload)`.
    pub signature: String,
    /// Hex-encoded 32-byte ed25519 public key of the vendor that
    /// issued this attestation.
    pub vendor_pubkey: String,
}

impl CertifiedAgent {
    /// Compute the canonical content hash for given bytecode. Matches
    /// the hash the attestation payload commits to.
    pub fn content_hash(bytecode: &[u8]) -> Hash {
        sha3_256(bytecode)
    }
}

/// Verifier for certified-agent attestations. Constructed with the
/// compiled-in Karoowa vendor key; every `verify` call checks that
/// the agent's `vendor_pubkey` matches, the signature is valid over
/// the canonical payload bytes, and the attestation has not expired.
pub struct AttestationVerifier {
    vendor_pubkey: [u8; 32],
}

impl AttestationVerifier {
    pub fn new(vendor_pubkey: [u8; 32]) -> Self {
        AttestationVerifier { vendor_pubkey }
    }

    /// Check a certified agent's attestation against wall-clock `now`.
    ///
    /// Returns `Ok(())` iff:
    /// 1. The declared vendor key matches the compiled-in key.
    /// 2. The signature verifies over the canonical payload bytes.
    /// 3. The attestation is not expired.
    ///
    /// Note that this does NOT verify that the agent binary the
    /// operator intends to load actually matches `content_hash`. That
    /// check is the caller's responsibility — see
    /// [`Registry::load_and_verify_bytecode`] for a helper that does
    /// it atomically.
    pub fn verify(&self, agent: &CertifiedAgent, now: u64) -> Result<(), MarketplaceError> {
        let vendor_pubkey = decode_32(&agent.vendor_pubkey)
            .map_err(|e| MarketplaceError::Malformed(format!("vendor_pubkey: {e}")))?;
        if vendor_pubkey != self.vendor_pubkey {
            return Err(MarketplaceError::UnknownVendor);
        }

        let sig_bytes = decode_64(&agent.signature)
            .map_err(|e| MarketplaceError::Malformed(format!("signature: {e}")))?;
        let payload_bytes = serde_json::to_vec(&agent.payload)
            .map_err(|e| MarketplaceError::Malformed(e.to_string()))?;
        let signature = Signature::from_parts(&sig_bytes, &vendor_pubkey)
            .map_err(|_| MarketplaceError::BadAttestation)?;
        signature
            .verify(&payload_bytes)
            .map_err(|_| MarketplaceError::BadAttestation)?;

        if now >= agent.payload.expires_at {
            return Err(MarketplaceError::Expired {
                expired_at: agent.payload.expires_at,
                now,
            });
        }
        Ok(())
    }
}

/// Helper used by tests and by the vendor tooling. Not used at
/// runtime on customer nodes — those only verify.
pub struct AttestationSigner<'a> {
    pub vendor_keypair: &'a Keypair,
}

impl<'a> AttestationSigner<'a> {
    pub fn sign(&self, payload: AgentAttestationPayload) -> CertifiedAgent {
        let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
        let sig = self.vendor_keypair.sign(&payload_bytes);
        CertifiedAgent {
            payload,
            signature: hex_encode(&sig.to_bytes()),
            vendor_pubkey: hex_encode(&sig.signer_public_key()),
        }
    }
}

/// Indexed set of certified agents. Keeps agents retrievable both by
/// `AgentId` (for lookups from the agent runtime) and by
/// `content_hash` (for reverse-lookup when verifying that a binary
/// on disk corresponds to a known-good attestation).
#[derive(Default)]
pub struct Registry {
    by_id: BTreeMap<AgentId, CertifiedAgent>,
    by_hash: BTreeMap<Hash, AgentId>,
}

impl Registry {
    pub fn new() -> Self {
        Registry::default()
    }

    /// Register a certified agent after a successful verification.
    pub fn register(
        &mut self,
        agent: CertifiedAgent,
        verifier: &AttestationVerifier,
        now: u64,
    ) -> Result<(), MarketplaceError> {
        verifier.verify(&agent, now)?;
        if self.by_id.contains_key(&agent.payload.id) {
            return Err(MarketplaceError::Duplicate(agent.payload.id.to_string()));
        }
        self.by_hash
            .insert(agent.payload.content_hash, agent.payload.id.clone());
        self.by_id.insert(agent.payload.id.clone(), agent);
        Ok(())
    }

    /// Look up a certified agent by id.
    pub fn get(&self, id: &AgentId) -> Option<&CertifiedAgent> {
        self.by_id.get(id)
    }

    /// Reverse lookup: given bytecode on disk, is there an
    /// attestation in this registry that matches?
    pub fn find_by_bytecode(&self, bytecode: &[u8]) -> Option<&CertifiedAgent> {
        let hash = CertifiedAgent::content_hash(bytecode);
        let id = self.by_hash.get(&hash)?;
        self.by_id.get(id)
    }

    /// Load a JSON file containing a single CertifiedAgent, verify
    /// it, and register it in one shot. Idempotent under duplicate
    /// loads — returns `Duplicate` the second time.
    pub fn load_and_register<P: AsRef<Path>>(
        &mut self,
        path: P,
        verifier: &AttestationVerifier,
        now: u64,
    ) -> Result<AgentId, MarketplaceError> {
        let bytes = fs::read(path.as_ref()).map_err(MarketplaceError::Io)?;
        let agent: CertifiedAgent = serde_json::from_slice(&bytes)
            .map_err(|e| MarketplaceError::Malformed(e.to_string()))?;
        let id = agent.payload.id.clone();
        self.register(agent, verifier, now)?;
        Ok(id)
    }

    /// Atomic "verify attestation + verify bytecode matches content
    /// hash". Used at agent-load time: the operator hands the
    /// runtime a bytecode blob and an attestation, and this function
    /// rejects the load if either check fails.
    pub fn load_and_verify_bytecode(
        &mut self,
        agent: CertifiedAgent,
        bytecode: &[u8],
        verifier: &AttestationVerifier,
        now: u64,
    ) -> Result<(), MarketplaceError> {
        let actual = CertifiedAgent::content_hash(bytecode);
        if actual != agent.payload.content_hash {
            return Err(MarketplaceError::ContentMismatch {
                expected: agent.payload.content_hash,
                actual,
            });
        }
        self.register(agent, verifier, now)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
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

    fn payload_for(id: &str, bytecode: &[u8]) -> AgentAttestationPayload {
        AgentAttestationPayload {
            id: AgentId::new(id),
            version: "1.0.0".into(),
            category: AgentCategory::Governance,
            content_hash: CertifiedAgent::content_hash(bytecode),
            issued_at: 1_000_000,
            expires_at: 2_000_000,
            vendor: "Karoowa".into(),
        }
    }

    #[test]
    fn signed_agent_verifies() {
        let keypair = Keypair::generate();
        let verifier = AttestationVerifier::new(keypair.public_key_bytes());
        let signer = AttestationSigner {
            vendor_keypair: &keypair,
        };
        let bytecode = b"agent wasm bytes";
        let agent = signer.sign(payload_for("com.karoowa.gov", bytecode));
        verifier.verify(&agent, 1_500_000).unwrap();
    }

    #[test]
    fn tampered_payload_rejected() {
        let keypair = Keypair::generate();
        let verifier = AttestationVerifier::new(keypair.public_key_bytes());
        let signer = AttestationSigner {
            vendor_keypair: &keypair,
        };
        let mut agent = signer.sign(payload_for("com.karoowa.gov", b"wasm"));
        // Change the category after signing.
        agent.payload.category = AgentCategory::Treasury;
        let err = verifier.verify(&agent, 1_500_000).unwrap_err();
        assert!(matches!(err, MarketplaceError::BadAttestation));
    }

    #[test]
    fn unknown_vendor_rejected() {
        let keypair = Keypair::generate();
        let verifier = AttestationVerifier::new([9u8; 32]); // different key
        let signer = AttestationSigner {
            vendor_keypair: &keypair,
        };
        let agent = signer.sign(payload_for("com.karoowa.gov", b"wasm"));
        let err = verifier.verify(&agent, 1_500_000).unwrap_err();
        assert!(matches!(err, MarketplaceError::UnknownVendor));
    }

    #[test]
    fn expired_attestation_rejected() {
        let keypair = Keypair::generate();
        let verifier = AttestationVerifier::new(keypair.public_key_bytes());
        let signer = AttestationSigner {
            vendor_keypair: &keypair,
        };
        let agent = signer.sign(payload_for("com.karoowa.gov", b"wasm"));
        let err = verifier.verify(&agent, 3_000_000).unwrap_err();
        assert!(matches!(err, MarketplaceError::Expired { .. }));
    }

    #[test]
    fn registry_index_and_reverse_lookup() {
        let keypair = Keypair::generate();
        let verifier = AttestationVerifier::new(keypair.public_key_bytes());
        let signer = AttestationSigner {
            vendor_keypair: &keypair,
        };
        let bytecode = b"specific wasm blob";
        let agent = signer.sign(payload_for("com.karoowa.optimizer", bytecode));
        let mut reg = Registry::new();
        reg.register(agent.clone(), &verifier, 1_500_000).unwrap();
        assert_eq!(reg.len(), 1);
        assert!(reg.get(&AgentId::new("com.karoowa.optimizer")).is_some());
        let found = reg.find_by_bytecode(bytecode).unwrap();
        assert_eq!(found.payload.id, AgentId::new("com.karoowa.optimizer"));
        assert!(reg.find_by_bytecode(b"wrong bytes").is_none());
    }

    #[test]
    fn duplicate_agent_rejected() {
        let keypair = Keypair::generate();
        let verifier = AttestationVerifier::new(keypair.public_key_bytes());
        let signer = AttestationSigner {
            vendor_keypair: &keypair,
        };
        let agent = signer.sign(payload_for("com.karoowa.gov", b"wasm"));
        let mut reg = Registry::new();
        reg.register(agent.clone(), &verifier, 1_500_000).unwrap();
        let err = reg.register(agent, &verifier, 1_500_000).unwrap_err();
        assert!(matches!(err, MarketplaceError::Duplicate(_)));
    }

    #[test]
    fn bytecode_mismatch_rejected() {
        let keypair = Keypair::generate();
        let verifier = AttestationVerifier::new(keypair.public_key_bytes());
        let signer = AttestationSigner {
            vendor_keypair: &keypair,
        };
        // Attestation commits to one blob; operator tries to load a
        // different one.
        let agent = signer.sign(payload_for("com.karoowa.gov", b"canonical wasm"));
        let mut reg = Registry::new();
        let err = reg
            .load_and_verify_bytecode(agent, b"different wasm", &verifier, 1_500_000)
            .unwrap_err();
        assert!(matches!(err, MarketplaceError::ContentMismatch { .. }));
    }
}
