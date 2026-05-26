//! Karoowa Enterprise — HSM abstraction and SoftHsm reference backend.
//!
//! # Rationale
//!
//! Validator signing keys are the highest-value secret on a Karoowa
//! node. Production deployments must store them in a Hardware
//! Security Module (HSM) so that an operator who compromises the host
//! filesystem cannot exfiltrate the key material — the HSM only ever
//! returns signatures, not private bytes.
//!
//! # Scope (v1.0)
//!
//! - [`HsmProvider`] trait — the abstract interface every backend
//!   implements. Three operations: `list_keys`, `public_key_of`,
//!   and `sign`. Key generation is an out-of-band ceremony and
//!   deliberately not in the trait.
//! - [`SoftHsm`] — a file-backed reference implementation that
//!   stores ed25519 keypairs on disk. Not a security control. Used
//!   by tests, CI, and operators who want to rehearse HSM flows
//!   without real hardware.
//!
//! Real AWS CloudHSM and YubiHSM 2 backends are deferred to v1.1 —
//! they require hardware in the audit lab and add heavy SDK
//! dependencies. Auditors review the trait surface and SoftHsm in
//! v1.0; real backends get audited as they land.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use karoowa_audit_log::{AuditAction, AuditDraft, AuditLog};
use karoowa_crypto::{Keypair, Signature};
use serde::{Deserialize, Serialize};

pub mod error;

pub use error::HsmError;

/// A stable identifier for a key inside an HSM. Chosen at key
/// generation time and used in every `sign` / `public_key_of` call.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KeyId(pub String);

impl KeyId {
    pub fn new(id: impl Into<String>) -> Self {
        KeyId(id.into())
    }
}

impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a particular key in the HSM is used for. Informational —
/// enforcement is the caller's responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyUsage {
    /// Validator consensus signing key.
    Validator,
    /// Operator identity key (RBAC principal signature).
    Operator,
    /// Bridge relayer signing key.
    BridgeRelayer,
    /// Generic key — used for tests and one-offs.
    Generic,
}

/// Metadata about a key the HSM holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyHandle {
    pub id: KeyId,
    pub usage: KeyUsage,
    /// Ed25519 public key bytes.
    pub public_key: [u8; 32],
}

/// Trait every HSM backend implements.
///
/// Implementations must be `Send + Sync` so the node can share a
/// single provider across signing tasks. Sign calls are expected to
/// be fast (sub-millisecond for SoftHsm; the real HSMs are typically
/// 1–10 ms).
pub trait HsmProvider: Send + Sync {
    /// Human-readable name (`"softhsm"`, `"aws-cloudhsm"`, `"yubihsm2"`).
    fn name(&self) -> &'static str;

    /// Enumerate every key currently held by this HSM.
    fn list_keys(&self) -> Result<Vec<KeyHandle>, HsmError>;

    /// Fetch the public key for a specific key id.
    fn public_key_of(&self, id: &KeyId) -> Result<[u8; 32], HsmError>;

    /// Sign a message with the key identified by `id`. The HSM
    /// returns a `karoowa_crypto::Signature` that verifies against
    /// the returned public key.
    fn sign(&self, id: &KeyId, message: &[u8]) -> Result<Signature, HsmError>;
}

/// Convenience wrapper: sign + emit an audit event.
pub fn sign_audited<H: HsmProvider + ?Sized>(
    hsm: &H,
    id: &KeyId,
    message: &[u8],
    principal: &str,
    reason: &str,
    audit: &AuditLog,
) -> Result<Signature, HsmError> {
    let result = hsm.sign(id, message);
    let (summary, metadata) = match &result {
        Ok(_) => (
            format!("hsm.sign ok key={id}"),
            serde_json::json!({
                "backend": hsm.name(),
                "key_id": id.to_string(),
                "reason": reason,
                "outcome": "ok",
            }),
        ),
        Err(e) => (
            format!("hsm.sign error key={id}"),
            serde_json::json!({
                "backend": hsm.name(),
                "key_id": id.to_string(),
                "reason": reason,
                "outcome": "error",
                "error": e.to_string(),
            }),
        ),
    };
    let draft = AuditDraft::new(AuditAction::HsmSign, principal, summary).with_metadata(metadata);
    let _ = audit.emit(draft);
    result
}

// -----------------------------------------------------------------
// SoftHsm — reference backend
// -----------------------------------------------------------------

/// File-backed reference HSM. **Not a security control.**
///
/// Stores keypairs in a JSON file on disk. Exists so that tests,
/// CI, and operators can rehearse HSM flows without real hardware.
/// Production deployments must use a real backend.
///
/// The store file format is:
///
/// ```json
/// {
///   "keys": [
///     {
///       "id": "validator-1",
///       "usage": "validator",
///       "secret_key": "hex…",
///       "public_key": "hex…"
///     }
///   ]
/// }
/// ```
pub struct SoftHsm {
    store_path: Option<PathBuf>,
    inner: Arc<Mutex<SoftHsmState>>,
}

#[derive(Serialize, Deserialize, Default)]
struct SoftHsmFile {
    keys: Vec<SoftHsmEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
struct SoftHsmEntry {
    id: String,
    usage: KeyUsage,
    /// Hex-encoded 32-byte ed25519 secret key.
    secret_key: String,
    /// Hex-encoded 32-byte ed25519 public key.
    public_key: String,
}

struct SoftHsmState {
    keys: BTreeMap<KeyId, (Keypair, KeyUsage)>,
}

impl SoftHsm {
    /// Build an in-memory SoftHsm with a pre-seeded key set. No disk
    /// persistence. Primarily for tests.
    pub fn in_memory(seed: Vec<(KeyId, KeyUsage, Keypair)>) -> Self {
        let mut keys = BTreeMap::new();
        for (id, usage, kp) in seed {
            keys.insert(id, (kp, usage));
        }
        SoftHsm {
            store_path: None,
            inner: Arc::new(Mutex::new(SoftHsmState { keys })),
        }
    }

    /// Load a SoftHsm from a JSON store file on disk. Missing files
    /// are treated as an empty store and not created until the first
    /// `add_generated_key` call.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, HsmError> {
        let path = path.as_ref().to_path_buf();
        let file = if path.exists() {
            let bytes = fs::read(&path).map_err(HsmError::Io)?;
            serde_json::from_slice::<SoftHsmFile>(&bytes)
                .map_err(|e| HsmError::Malformed(e.to_string()))?
        } else {
            SoftHsmFile::default()
        };

        let mut keys = BTreeMap::new();
        for entry in file.keys {
            let secret_bytes = decode_32(&entry.secret_key)
                .map_err(|e| HsmError::Malformed(format!("secret_key: {e}")))?;
            let keypair = Keypair::from_seed(&secret_bytes);
            keys.insert(KeyId(entry.id), (keypair, entry.usage));
        }

        Ok(SoftHsm {
            store_path: Some(path),
            inner: Arc::new(Mutex::new(SoftHsmState { keys })),
        })
    }

    /// Generate a new ed25519 keypair, add it to the store, and
    /// persist if this SoftHsm was opened from disk.
    pub fn add_generated_key(&self, id: KeyId, usage: KeyUsage) -> Result<KeyHandle, HsmError> {
        let keypair = Keypair::generate();
        let handle = KeyHandle {
            id: id.clone(),
            usage,
            public_key: keypair.public_key_bytes(),
        };
        {
            let mut state = self
                .inner
                .lock()
                .map_err(|_| HsmError::Internal("mutex poisoned".into()))?;
            if state.keys.contains_key(&id) {
                return Err(HsmError::DuplicateKey(id.to_string()));
            }
            state.keys.insert(id, (keypair, usage));
        }
        self.persist()?;
        Ok(handle)
    }

    fn persist(&self) -> Result<(), HsmError> {
        let Some(path) = &self.store_path else {
            return Ok(());
        };
        let state = self
            .inner
            .lock()
            .map_err(|_| HsmError::Internal("mutex poisoned".into()))?;
        let file = SoftHsmFile {
            keys: state
                .keys
                .iter()
                .map(|(id, (keypair, usage))| SoftHsmEntry {
                    id: id.to_string(),
                    usage: *usage,
                    secret_key: hex_encode(&keypair.private_key_bytes()),
                    public_key: hex_encode(&keypair.public_key_bytes()),
                })
                .collect(),
        };
        let bytes =
            serde_json::to_vec_pretty(&file).map_err(|e| HsmError::Malformed(e.to_string()))?;
        fs::write(path, bytes).map_err(HsmError::Io)?;
        Ok(())
    }
}

impl HsmProvider for SoftHsm {
    fn name(&self) -> &'static str {
        "softhsm"
    }

    fn list_keys(&self) -> Result<Vec<KeyHandle>, HsmError> {
        let state = self
            .inner
            .lock()
            .map_err(|_| HsmError::Internal("mutex poisoned".into()))?;
        Ok(state
            .keys
            .iter()
            .map(|(id, (kp, usage))| KeyHandle {
                id: id.clone(),
                usage: *usage,
                public_key: kp.public_key_bytes(),
            })
            .collect())
    }

    fn public_key_of(&self, id: &KeyId) -> Result<[u8; 32], HsmError> {
        let state = self
            .inner
            .lock()
            .map_err(|_| HsmError::Internal("mutex poisoned".into()))?;
        state
            .keys
            .get(id)
            .map(|(kp, _)| kp.public_key_bytes())
            .ok_or_else(|| HsmError::UnknownKey(id.to_string()))
    }

    fn sign(&self, id: &KeyId, message: &[u8]) -> Result<Signature, HsmError> {
        let state = self
            .inner
            .lock()
            .map_err(|_| HsmError::Internal("mutex poisoned".into()))?;
        let (keypair, _) = state
            .keys
            .get(id)
            .ok_or_else(|| HsmError::UnknownKey(id.to_string()))?;
        Ok(keypair.sign(message))
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_32(hex: &str) -> Result<[u8; 32], String> {
    let s = hex.strip_prefix("0x").unwrap_or(hex);
    let bytes: Result<Vec<u8>, String> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect();
    let bytes = bytes?;
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_audit_log::MemorySink;

    #[test]
    fn in_memory_list_and_sign() {
        let kp = Keypair::generate();
        let id = KeyId::new("validator-1");
        let hsm = SoftHsm::in_memory(vec![(id.clone(), KeyUsage::Validator, kp)]);

        let handles = hsm.list_keys().unwrap();
        assert_eq!(handles.len(), 1);
        assert_eq!(handles[0].id, id);
        assert_eq!(handles[0].usage, KeyUsage::Validator);

        let msg = b"block 42 precommit";
        let sig = hsm.sign(&id, msg).unwrap();
        sig.verify(msg).unwrap();
    }

    #[test]
    fn unknown_key_rejected() {
        let hsm = SoftHsm::in_memory(vec![]);
        let err = hsm.sign(&KeyId::new("missing"), b"x").unwrap_err();
        assert!(matches!(err, HsmError::UnknownKey(_)));
    }

    #[test]
    fn public_key_matches_sign_output() {
        let kp = Keypair::generate();
        let expected_pk = kp.public_key_bytes();
        let id = KeyId::new("k");
        let hsm = SoftHsm::in_memory(vec![(id.clone(), KeyUsage::Generic, kp)]);
        assert_eq!(hsm.public_key_of(&id).unwrap(), expected_pk);
    }

    #[test]
    fn disk_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("softhsm.json");

        let hsm = SoftHsm::load(&path).unwrap();
        let handle = hsm
            .add_generated_key(KeyId::new("relayer"), KeyUsage::BridgeRelayer)
            .unwrap();
        let msg = b"packet commitment";
        let sig = hsm.sign(&handle.id, msg).unwrap();
        sig.verify(msg).unwrap();

        // Reopen — key should persist with identical public key.
        let reopened = SoftHsm::load(&path).unwrap();
        let pk_after = reopened.public_key_of(&handle.id).unwrap();
        assert_eq!(pk_after, handle.public_key);
        let sig2 = reopened.sign(&handle.id, msg).unwrap();
        sig2.verify(msg).unwrap();
    }

    #[test]
    fn duplicate_key_id_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("softhsm.json");
        let hsm = SoftHsm::load(&path).unwrap();
        hsm.add_generated_key(KeyId::new("dup"), KeyUsage::Generic)
            .unwrap();
        let err = hsm
            .add_generated_key(KeyId::new("dup"), KeyUsage::Generic)
            .unwrap_err();
        assert!(matches!(err, HsmError::DuplicateKey(_)));
    }

    #[test]
    fn sign_audited_emits_event() {
        let kp = Keypair::generate();
        let id = KeyId::new("k");
        let hsm = SoftHsm::in_memory(vec![(id.clone(), KeyUsage::Validator, kp)]);
        let log = AuditLog::new(Box::new(MemorySink::new()));

        let sig = sign_audited(
            &hsm,
            &id,
            b"precommit h=1",
            "operator@karoowa",
            "bft precommit",
            &log,
        )
        .unwrap();
        sig.verify(b"precommit h=1").unwrap();
        assert_eq!(log.next_sequence(), 1);
    }

    #[test]
    fn missing_store_file_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does-not-exist.json");
        let hsm = SoftHsm::load(&path).unwrap();
        assert!(hsm.list_keys().unwrap().is_empty());
    }
}
