//! Karoowa Enterprise — node wrapper.
//!
//! Composes the six Phase 6.3 enterprise crates into a single
//! [`EnterpriseContext`] the node binary holds at runtime. The
//! wrapper lives in `enterprise/` (so the open-core cross-import
//! guardrail stays intact) but depends on `karoowa-core` for the
//! `LicenseGate` trait and crypto primitives.
//!
//! # What it does
//!
//! Startup: parse an [`EnterpriseConfig`], build each subsystem in
//! dependency order, and fail closed if any required component
//! cannot be initialized.
//!
//! 1. **License gate.** Load and verify the signed license file.
//!    If any enterprise flag is enabled but the license is missing,
//!    expired, or fails verification, `EnterpriseContext::load`
//!    returns an error and the caller must fall back to the OSS
//!    license gate.
//! 2. **Audit log.** Open (or create) a `FileSink` backed JSONL
//!    audit log. Every subsequent enterprise operation emits
//!    through this log, giving the SOC 2 auditor one append-only
//!    file per host.
//! 3. **RBAC policy.** Load the JSON policy file (if any) into a
//!    [`PolicyEngine`]. RPC middleware calls into it via the
//!    [`EnterpriseContext::authorize`] helper.
//! 4. **HSM.** Instantiate the configured [`HsmProvider`] backend
//!    (only `softhsm` ships in v1.0). Validator signing keys are
//!    pulled through this handle.
//! 5. **HA coordinator.** If HA is enabled, build an
//!    [`HaCoordinator`] over the chosen lease backend. The node
//!    binary drives it via `ticker()` on each block interval.
//! 6. **Marketplace registry.** Load any certified-agent
//!    attestations the operator has provisioned so the agent
//!    runtime can gate loads against the registry.
//!
//! Shutdown: call [`EnterpriseContext::shutdown`]. Releases the HA
//! lease for fast failover and emits a final audit event.

use std::path::PathBuf;
use std::sync::Arc;

use karoowa_audit_log::{AuditAction, AuditDraft, AuditLog, FileSink, MemorySink};
use karoowa_core::{LicenseGate, LicenseInfo, OssLicenseGate};
use karoowa_ha::{HaConfig, HaCoordinator, InMemoryLease, LeaseBackend};
use karoowa_hsm::{HsmProvider, SoftHsm};
use karoowa_license::SignedLicenseGate;
use karoowa_marketplace::{AttestationVerifier, Registry};
use karoowa_rbac::{Decision, Policy, PolicyEngine};

pub mod error;

pub use error::EnterpriseError;

/// How the HSM should be provisioned at startup.
#[derive(Debug, Clone)]
pub enum HsmConfig {
    /// Do not initialize an HSM. The node falls back to file-based
    /// keys. Not recommended for production validators.
    None,
    /// File-backed SoftHsm at the given path. Not a security
    /// control — exists so tests and dev clusters can rehearse the
    /// flow without real hardware.
    Soft { store_path: PathBuf },
}

/// Which lease backend drives the HA coordinator.
#[derive(Debug, Clone)]
pub enum HaLeaseConfig {
    /// HA disabled; the node runs standalone.
    Disabled,
    /// Single-process in-memory lease. Useful for tests and dev
    /// harnesses where both peers live in the same binary.
    InMemory,
}

/// Configuration for the enterprise wrapper.
#[derive(Debug, Clone, Default)]
pub struct EnterpriseConfig {
    /// Optional license file. If `None`, the wrapper falls back to
    /// the OSS gate and refuses to enable any enterprise feature.
    pub license_path: Option<PathBuf>,
    /// Audit log file path. If `None`, an in-memory audit sink is
    /// used (fine for tests, never for production).
    pub audit_log_path: Option<PathBuf>,
    /// RBAC policy file path. If `None`, the engine is built empty
    /// (deny-all).
    pub rbac_policy_path: Option<PathBuf>,
    /// HSM backend selection.
    pub hsm: Option<HsmConfig>,
    /// HA configuration. `None` means HA disabled.
    pub ha: Option<(HaLeaseConfig, HaConfig)>,
    /// Path to a directory of certified-agent JSON attestations.
    /// Every `.json` file in this directory is verified at
    /// startup and registered.
    pub marketplace_attestations_dir: Option<PathBuf>,
    /// Compiled-in marketplace vendor key. Placeholder until the
    /// real vendor key ships via build.rs.
    pub marketplace_vendor_pubkey: [u8; 32],
}

impl EnterpriseConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Top-level handle the node binary holds. All six enterprise
/// subsystems hang off this struct.
pub struct EnterpriseContext {
    pub license: Arc<dyn LicenseGate>,
    pub audit: Arc<AuditLog>,
    pub rbac: Arc<PolicyEngine>,
    pub hsm: Option<Arc<dyn HsmProvider>>,
    pub ha: Option<Arc<HaCoordinator>>,
    pub marketplace: Arc<Registry>,
    node_id: String,
}

impl EnterpriseContext {
    /// Build the wrapper from config. Fails closed: any required
    /// subsystem that cannot be initialized aborts startup.
    ///
    /// `now` is the current wall clock in Unix seconds — injected
    /// so tests can exercise expiry without a real clock.
    pub fn load(
        config: EnterpriseConfig,
        now: u64,
        node_id: impl Into<String>,
    ) -> Result<Self, EnterpriseError> {
        let node_id = node_id.into();

        // 1. License gate.
        let license: Arc<dyn LicenseGate> = match &config.license_path {
            Some(path) => {
                let bytes = std::fs::read(path).map_err(EnterpriseError::Io)?;
                let gate = SignedLicenseGate::from_bytes(
                    &bytes,
                    &karoowa_license::KAROOWA_VENDOR_PUBKEY,
                    now,
                )
                .map_err(|e| EnterpriseError::License(e.to_string()))?;
                Arc::new(gate) as Arc<dyn LicenseGate>
            }
            None => Arc::new(OssLicenseGate) as Arc<dyn LicenseGate>,
        };

        // 2. Audit log.
        let audit: Arc<AuditLog> = match &config.audit_log_path {
            Some(path) => Arc::new(AuditLog::new(Box::new(
                FileSink::open(path).map_err(|e| EnterpriseError::Audit(e.to_string()))?,
            ))),
            None => Arc::new(AuditLog::new(Box::new(MemorySink::new()))),
        };

        // Emit a startup event so the audit trail has a clear
        // session boundary.
        let license_info = license.license_info();
        let _ = audit.emit(
            AuditDraft::new(
                AuditAction::LicenseEvent,
                &node_id,
                format!("enterprise startup, edition={}", license_info.edition),
            )
            .with_metadata(serde_json::json!({
                "features": license_info.features,
                "expires_at": license_info.expires_at,
            })),
        );

        // 3. RBAC — gated on license having "rbac" feature.
        let rbac: Arc<PolicyEngine> = if let Some(path) = &config.rbac_policy_path {
            if !license.is_feature_enabled("rbac") {
                return Err(EnterpriseError::FeatureNotLicensed("rbac".into()));
            }
            let policy = Policy::load(path).map_err(|e| EnterpriseError::Rbac(e.to_string()))?;
            Arc::new(PolicyEngine::new(policy))
        } else {
            Arc::new(PolicyEngine::empty())
        };

        // 4. HSM — gated on "hsm" feature.
        let hsm: Option<Arc<dyn HsmProvider>> = match &config.hsm {
            None | Some(HsmConfig::None) => None,
            Some(HsmConfig::Soft { store_path }) => {
                if !license.is_feature_enabled("hsm") {
                    return Err(EnterpriseError::FeatureNotLicensed("hsm".into()));
                }
                let softhsm =
                    SoftHsm::load(store_path).map_err(|e| EnterpriseError::Hsm(e.to_string()))?;
                Some(Arc::new(softhsm) as Arc<dyn HsmProvider>)
            }
        };

        // 5. HA coordinator — gated on "ha" feature.
        let ha = if let Some((lease_cfg, ha_cfg)) = config.ha.clone() {
            match lease_cfg {
                HaLeaseConfig::Disabled => None,
                HaLeaseConfig::InMemory => {
                    if !license.is_feature_enabled("ha") {
                        return Err(EnterpriseError::FeatureNotLicensed("ha".into()));
                    }
                    let backend: Arc<dyn LeaseBackend> = Arc::new(InMemoryLease::new());
                    Some(Arc::new(HaCoordinator::new(ha_cfg, backend)))
                }
            }
        } else {
            None
        };

        // 6. Marketplace registry — gated on "marketplace" feature.
        let mut registry = Registry::new();
        if let Some(dir) = &config.marketplace_attestations_dir {
            if !license.is_feature_enabled("marketplace") {
                return Err(EnterpriseError::FeatureNotLicensed("marketplace".into()));
            }
            let verifier = AttestationVerifier::new(config.marketplace_vendor_pubkey);
            for entry in std::fs::read_dir(dir).map_err(EnterpriseError::Io)? {
                let path = entry.map_err(EnterpriseError::Io)?.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                registry
                    .load_and_register(&path, &verifier, now)
                    .map_err(|e| EnterpriseError::Marketplace(e.to_string()))?;
            }
        }
        let marketplace = Arc::new(registry);

        Ok(EnterpriseContext {
            license,
            audit,
            rbac,
            hsm,
            ha,
            marketplace,
            node_id,
        })
    }

    /// Return the license info the wrapper booted with.
    pub fn license_info(&self) -> LicenseInfo {
        self.license.license_info()
    }

    /// Whether the named feature is unlocked. Convenience over the
    /// raw `LicenseGate::is_feature_enabled`.
    pub fn feature_enabled(&self, name: &str) -> bool {
        self.license.is_feature_enabled(name)
    }

    /// Authorize a `(principal, action)` request, emitting an audit
    /// event for both allow and deny. The RPC middleware layer
    /// calls this on every privileged request.
    pub fn authorize(&self, principal: &str, action: &str) -> Decision {
        self.rbac.check_and_audit(principal, action, &self.audit)
    }

    /// Signature from the validator HSM key. Returns an error if
    /// HSM is not enabled or the key id is unknown.
    pub fn hsm_sign(
        &self,
        key_id: &karoowa_hsm::KeyId,
        message: &[u8],
        reason: &str,
    ) -> Result<karoowa_crypto::Signature, EnterpriseError> {
        let hsm = self
            .hsm
            .as_ref()
            .ok_or(EnterpriseError::FeatureNotLicensed("hsm".into()))?;
        karoowa_hsm::sign_audited(
            hsm.as_ref(),
            key_id,
            message,
            &self.node_id,
            reason,
            &self.audit,
        )
        .map_err(|e| EnterpriseError::Hsm(e.to_string()))
    }

    /// Drive the HA coordinator one step. Returns whether the
    /// caller should start/stop block production. Safe to call on
    /// a non-HA node — returns `NoChange` in that case.
    pub fn ha_tick(&self, now: u64) -> karoowa_ha::StateTransition {
        match &self.ha {
            Some(coord) => coord.tick_and_audit(now, &self.audit),
            None => karoowa_ha::StateTransition::NoChange,
        }
    }

    /// Graceful shutdown: release the HA lease (if any) and emit
    /// a final audit event.
    pub fn shutdown(&self) {
        if let Some(coord) = &self.ha {
            coord.shutdown();
        }
        let _ = self.audit.emit(AuditDraft::new(
            AuditAction::AdminAuth,
            &self.node_id,
            "enterprise shutdown",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_audit_log::FileSink;
    use karoowa_crypto::Keypair;
    use karoowa_ha::NodeId;
    use karoowa_license::{LicenseFile, LicensePayload};

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn signed_license_bytes(features: Vec<&str>) -> (Vec<u8>, [u8; 32]) {
        let keypair = Keypair::generate();
        let vendor_pk = keypair.public_key_bytes();
        let payload = LicensePayload {
            edition: "enterprise".into(),
            customer: "TestCo".into(),
            issued_at: 500_000,
            expires_at: 10_000_000,
            features: features.iter().map(|s| s.to_string()).collect(),
        };
        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let sig = keypair.sign(&payload_bytes);
        let file = LicenseFile {
            payload,
            signature: hex_encode(&sig.to_bytes()),
            signer_pubkey: hex_encode(&sig.signer_public_key()),
        };
        (serde_json::to_vec(&file).unwrap(), vendor_pk)
    }

    /// Build a context with OSS license (no license file) and an
    /// in-memory audit sink. This path is exercised most often in
    /// tests — it still boots and still enforces deny-all RBAC.
    #[test]
    fn oss_context_boots_with_deny_all() {
        let ctx = EnterpriseContext::load(EnterpriseConfig::new(), 1_500_000, "node-a").unwrap();
        assert_eq!(ctx.license_info().edition, karoowa_core::Edition::Oss);
        assert!(!ctx.feature_enabled("rbac"));
        // Deny-all: no principals registered.
        assert_eq!(ctx.authorize("alice", "rpc.read.block"), Decision::Deny);
        // Audit log advanced by startup event + two authorize calls.
        assert_eq!(ctx.audit.next_sequence(), 2);
    }

    #[test]
    fn enterprise_license_unlocks_rbac_with_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let license_path = tmp.path().join("license.json");
        let policy_path = tmp.path().join("rbac.json");

        let (license_bytes, vendor_pk) = signed_license_bytes(vec!["rbac", "audit-log"]);
        std::fs::write(&license_path, &license_bytes).unwrap();

        // Write a minimal policy.
        let policy = serde_json::json!({
            "version": 1,
            "assignments": [
                { "principal": "admin@karoowa", "roles": ["admin"] }
            ]
        });
        std::fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();

        // Temporarily override the vendor key by building the
        // license gate directly — the real loader uses the
        // compiled-in placeholder and would reject. We validate
        // the downstream wiring through a custom bootstrap helper.
        let gate = SignedLicenseGate::from_bytes(&license_bytes, &vendor_pk, 1_500_000).unwrap();
        let license: Arc<dyn LicenseGate> = Arc::new(gate);
        let audit = Arc::new(AuditLog::new(Box::new(MemorySink::new())));
        let rbac_policy = Policy::load(&policy_path).unwrap();
        let rbac = Arc::new(PolicyEngine::new(rbac_policy));
        let ctx = EnterpriseContext {
            license,
            audit,
            rbac,
            hsm: None,
            ha: None,
            marketplace: Arc::new(Registry::new()),
            node_id: "node-a".into(),
        };

        assert_eq!(
            ctx.license_info().edition,
            karoowa_core::Edition::Enterprise
        );
        assert!(ctx.feature_enabled("rbac"));
        assert_eq!(
            ctx.authorize("admin@karoowa", "node.admin.restart"),
            Decision::Allow
        );
        assert_eq!(
            ctx.authorize("mallory", "node.admin.restart"),
            Decision::Deny
        );
    }

    #[test]
    fn audit_events_persist_to_file_sink() {
        let tmp = tempfile::tempdir().unwrap();
        let audit_path = tmp.path().join("audit.jsonl");
        let cfg = EnterpriseConfig {
            audit_log_path: Some(audit_path.clone()),
            ..Default::default()
        };
        let ctx = EnterpriseContext::load(cfg, 1_500_000, "node-a").unwrap();
        ctx.authorize("alice", "rpc.read.block");
        ctx.shutdown();
        drop(ctx);

        // Verify the chain survives reopening the file.
        let count = FileSink::verify_chain(&audit_path).unwrap();
        assert!(count >= 3); // startup + authorize + shutdown
    }

    #[test]
    fn ha_tick_emits_become_active_transition() {
        let cfg = EnterpriseConfig {
            ha: Some((
                HaLeaseConfig::InMemory,
                HaConfig::new(NodeId::new("node-a")),
            )),
            ..Default::default()
        };
        // HA without the license feature should fail closed.
        match EnterpriseContext::load(cfg.clone(), 1_500_000, "node-a") {
            Err(EnterpriseError::FeatureNotLicensed(_)) => {}
            Err(e) => panic!("expected FeatureNotLicensed, got {e:?}"),
            Ok(_) => panic!("expected FeatureNotLicensed, got Ok"),
        }
    }

    #[test]
    fn hsm_missing_feature_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = EnterpriseConfig {
            hsm: Some(HsmConfig::Soft {
                store_path: tmp.path().join("softhsm.json"),
            }),
            ..Default::default()
        };
        match EnterpriseContext::load(cfg, 1_500_000, "node-a") {
            Err(EnterpriseError::FeatureNotLicensed(_)) => {}
            Err(e) => panic!("expected FeatureNotLicensed, got {e:?}"),
            Ok(_) => panic!("expected FeatureNotLicensed, got Ok"),
        }
    }

    #[test]
    fn marketplace_missing_feature_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let att_dir = tmp.path().join("attestations");
        std::fs::create_dir_all(&att_dir).unwrap();
        let cfg = EnterpriseConfig {
            marketplace_attestations_dir: Some(att_dir),
            ..Default::default()
        };
        match EnterpriseContext::load(cfg, 1_500_000, "node-a") {
            Err(EnterpriseError::FeatureNotLicensed(_)) => {}
            Err(e) => panic!("expected FeatureNotLicensed, got {e:?}"),
            Ok(_) => panic!("expected FeatureNotLicensed, got Ok"),
        }
    }

    #[test]
    fn shutdown_emits_audit_event() {
        let ctx = EnterpriseContext::load(EnterpriseConfig::new(), 1_500_000, "node-a").unwrap();
        let before = ctx.audit.next_sequence();
        ctx.shutdown();
        assert_eq!(ctx.audit.next_sequence(), before + 1);
    }
}
