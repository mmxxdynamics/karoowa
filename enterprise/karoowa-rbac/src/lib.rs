//! Karoowa Enterprise — role-based access control.
//!
//! # Model
//!
//! - **Principal** — an identity string (operator email, node address,
//!   service account). Free-form; the integrator picks the namespace.
//! - **Role** — a named bundle of capabilities. Four ship out of the
//!   box ([`Role::Admin`], [`Role::Operator`], [`Role::Deployer`],
//!   [`Role::Reader`]) covering the node-ops-and-deployer use case
//!   called out in dev plan T6.3.1.
//! - **Action** — a dot-namespaced capability string, e.g.
//!   `"node.admin.restart"`, `"contract.deploy"`, `"rpc.read.block"`.
//!   The set of known actions lives in [`actions`].
//! - **Policy** — a map from principal → set of roles. Loaded from a
//!   JSON file at node startup.
//! - **PolicyEngine** — walks the policy, expands roles into action
//!   sets, and answers [`PolicyEngine::check`].
//!
//! # Integration
//!
//! Because `core/` may not import from `enterprise/` (open-core
//! guardrail — see `scripts/check-cross-imports.sh`), the node binary
//! integrates RBAC via a thin middleware layer in a downstream
//! enterprise node wrapper crate (landing in Phase 6.3.x). This crate
//! ships only the library + policy file parser; it is side-effect
//! free and deterministic.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use karoowa_audit_log::{AuditAction, AuditDraft, AuditLog};
use serde::{Deserialize, Serialize};

pub mod error;

pub use error::RbacError;

/// The four canonical Karoowa RBAC roles. Any given principal may be
/// assigned more than one role; their effective capability set is the
/// union of the roles' actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Full privileges: every action is allowed.
    Admin,
    /// Can operate the node: start/stop, reload config, rotate keys,
    /// inspect state. Cannot deploy contracts or modify the RBAC
    /// policy itself.
    Operator,
    /// Can deploy and upgrade contracts. Cannot touch node config or
    /// validator keys.
    Deployer,
    /// Read-only: RPC queries, log export, metrics scrape. Cannot
    /// mutate state.
    Reader,
}

impl Role {
    /// The action namespace this role grants. A trailing `*` means
    /// "any action under this prefix".
    pub fn allowed_actions(&self) -> &'static [&'static str] {
        match self {
            Role::Admin => &["*"],
            Role::Operator => &[
                "node.admin.*",
                "rpc.read.*",
                "rpc.subscribe.*",
                "audit.read.*",
            ],
            Role::Deployer => &[
                "contract.deploy",
                "contract.upgrade",
                "contract.call",
                "rpc.read.*",
            ],
            Role::Reader => &["rpc.read.*", "rpc.subscribe.*", "audit.read.*"],
        }
    }

    /// Human-readable display name used in audit logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "Admin",
            Role::Operator => "Operator",
            Role::Deployer => "Deployer",
            Role::Reader => "Reader",
        }
    }
}

/// A role assignment: one principal can hold multiple roles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalRoles {
    pub principal: String,
    pub roles: BTreeSet<Role>,
}

/// The full RBAC policy as stored on disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Policy {
    /// Policy schema version — bumped on any breaking change to the
    /// file format.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Principal-to-roles assignments.
    pub assignments: Vec<PrincipalRoles>,
}

fn default_version() -> u32 {
    1
}

impl Policy {
    /// Parse a policy from JSON bytes.
    pub fn from_json(bytes: &[u8]) -> Result<Self, RbacError> {
        serde_json::from_slice(bytes).map_err(|e| RbacError::Malformed(e.to_string()))
    }

    /// Load a policy from a JSON file on disk.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, RbacError> {
        let bytes = fs::read(path.as_ref()).map_err(RbacError::Io)?;
        Self::from_json(&bytes)
    }
}

/// The RBAC evaluator. Cheap to build from a [`Policy`]; re-evaluates
/// in constant time per check.
pub struct PolicyEngine {
    /// Map from principal → effective role set.
    assignments: BTreeMap<String, BTreeSet<Role>>,
}

/// Outcome of a single access check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl PolicyEngine {
    /// Build an engine from a parsed policy.
    pub fn new(policy: Policy) -> Self {
        let mut assignments: BTreeMap<String, BTreeSet<Role>> = BTreeMap::new();
        for entry in policy.assignments {
            assignments
                .entry(entry.principal)
                .or_default()
                .extend(entry.roles);
        }
        PolicyEngine { assignments }
    }

    /// Build an engine with no assignments (deny-all).
    pub fn empty() -> Self {
        PolicyEngine {
            assignments: BTreeMap::new(),
        }
    }

    /// Returns `Decision::Allow` iff `principal` holds any role whose
    /// `allowed_actions()` matches `action`. Unknown principals are
    /// denied. The matching rules are:
    ///
    /// - `"*"` matches every action.
    /// - `"foo.bar.*"` matches `"foo.bar.baz"` and `"foo.bar.qux"`
    ///   but not `"foo.other.baz"`.
    /// - Otherwise the match is exact.
    pub fn check(&self, principal: &str, action: &str) -> Decision {
        let Some(roles) = self.assignments.get(principal) else {
            return Decision::Deny;
        };
        for role in roles {
            for pattern in role.allowed_actions() {
                if action_matches(pattern, action) {
                    return Decision::Allow;
                }
            }
        }
        Decision::Deny
    }

    /// Convenience: like `check` but also emits an RBAC audit event
    /// to the given log. Use from RPC middleware so every denied
    /// request is traceable.
    pub fn check_and_audit(&self, principal: &str, action: &str, audit: &AuditLog) -> Decision {
        let decision = self.check(principal, action);
        let summary = match decision {
            Decision::Allow => format!("allow {action}"),
            Decision::Deny => format!("deny {action}"),
        };
        let draft = AuditDraft::new(AuditAction::AdminAuth, principal, summary).with_metadata(
            serde_json::json!({
                "action": action,
                "decision": match decision {
                    Decision::Allow => "allow",
                    Decision::Deny => "deny",
                },
            }),
        );
        let _ = audit.emit(draft);
        decision
    }

    /// Return every role assigned to the given principal.
    pub fn roles_of(&self, principal: &str) -> Vec<Role> {
        self.assignments
            .get(principal)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Return the number of principals with at least one role.
    pub fn principal_count(&self) -> usize {
        self.assignments.len()
    }
}

/// Match an action against a pattern. `"*"` matches everything;
/// `"prefix.*"` matches anything under that prefix; otherwise exact.
fn action_matches(pattern: &str, action: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        return action == prefix || action.starts_with(&format!("{prefix}."));
    }
    pattern == action
}

/// Canonical action namespace. These names are used across all
/// enterprise integrations so ops tooling can reason about policy
/// coverage uniformly.
pub mod actions {
    // Node admin
    pub const NODE_ADMIN_RESTART: &str = "node.admin.restart";
    pub const NODE_ADMIN_RELOAD_CONFIG: &str = "node.admin.reload_config";
    pub const NODE_ADMIN_ROTATE_KEYS: &str = "node.admin.rotate_keys";
    pub const NODE_ADMIN_UPDATE_PEERS: &str = "node.admin.update_peers";

    // Contract operations
    pub const CONTRACT_DEPLOY: &str = "contract.deploy";
    pub const CONTRACT_UPGRADE: &str = "contract.upgrade";
    pub const CONTRACT_CALL: &str = "contract.call";

    // RPC (all read-only are under rpc.read.*)
    pub const RPC_READ_BLOCK: &str = "rpc.read.block";
    pub const RPC_READ_TX: &str = "rpc.read.tx";
    pub const RPC_READ_STATE: &str = "rpc.read.state";
    pub const RPC_SUBSCRIBE_BLOCKS: &str = "rpc.subscribe.blocks";

    // Audit log
    pub const AUDIT_READ: &str = "audit.read.events";
    pub const AUDIT_EXPORT: &str = "audit.read.export";
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_audit_log::{AuditLog, MemorySink};

    fn make_policy() -> Policy {
        Policy {
            version: 1,
            assignments: vec![
                PrincipalRoles {
                    principal: "alice@karoowa".into(),
                    roles: [Role::Admin].into_iter().collect(),
                },
                PrincipalRoles {
                    principal: "bob@karoowa".into(),
                    roles: [Role::Operator].into_iter().collect(),
                },
                PrincipalRoles {
                    principal: "dev-ci".into(),
                    roles: [Role::Deployer].into_iter().collect(),
                },
                PrincipalRoles {
                    principal: "monitor".into(),
                    roles: [Role::Reader].into_iter().collect(),
                },
            ],
        }
    }

    #[test]
    fn admin_can_do_everything() {
        let engine = PolicyEngine::new(make_policy());
        assert_eq!(
            engine.check("alice@karoowa", actions::NODE_ADMIN_RESTART),
            Decision::Allow
        );
        assert_eq!(
            engine.check("alice@karoowa", actions::CONTRACT_DEPLOY),
            Decision::Allow
        );
        assert_eq!(
            engine.check("alice@karoowa", "made.up.action"),
            Decision::Allow
        );
    }

    #[test]
    fn operator_can_rotate_keys_but_not_deploy() {
        let engine = PolicyEngine::new(make_policy());
        assert_eq!(
            engine.check("bob@karoowa", actions::NODE_ADMIN_ROTATE_KEYS),
            Decision::Allow
        );
        assert_eq!(
            engine.check("bob@karoowa", actions::CONTRACT_DEPLOY),
            Decision::Deny
        );
    }

    #[test]
    fn deployer_can_deploy_but_not_restart_node() {
        let engine = PolicyEngine::new(make_policy());
        assert_eq!(
            engine.check("dev-ci", actions::CONTRACT_DEPLOY),
            Decision::Allow
        );
        assert_eq!(
            engine.check("dev-ci", actions::CONTRACT_UPGRADE),
            Decision::Allow
        );
        assert_eq!(
            engine.check("dev-ci", actions::NODE_ADMIN_RESTART),
            Decision::Deny
        );
    }

    #[test]
    fn reader_is_read_only() {
        let engine = PolicyEngine::new(make_policy());
        assert_eq!(
            engine.check("monitor", actions::RPC_READ_BLOCK),
            Decision::Allow
        );
        assert_eq!(
            engine.check("monitor", actions::RPC_SUBSCRIBE_BLOCKS),
            Decision::Allow
        );
        assert_eq!(
            engine.check("monitor", actions::CONTRACT_DEPLOY),
            Decision::Deny
        );
        assert_eq!(
            engine.check("monitor", actions::NODE_ADMIN_RESTART),
            Decision::Deny
        );
    }

    #[test]
    fn unknown_principal_is_denied() {
        let engine = PolicyEngine::new(make_policy());
        assert_eq!(
            engine.check("mallory", actions::RPC_READ_BLOCK),
            Decision::Deny
        );
    }

    #[test]
    fn empty_engine_denies_all() {
        let engine = PolicyEngine::empty();
        assert_eq!(
            engine.check("alice@karoowa", actions::RPC_READ_BLOCK),
            Decision::Deny
        );
    }

    #[test]
    fn multi_role_principal_gets_union() {
        let policy = Policy {
            version: 1,
            assignments: vec![PrincipalRoles {
                principal: "power-dev".into(),
                roles: [Role::Deployer, Role::Operator].into_iter().collect(),
            }],
        };
        let engine = PolicyEngine::new(policy);
        assert_eq!(
            engine.check("power-dev", actions::CONTRACT_DEPLOY),
            Decision::Allow
        );
        assert_eq!(
            engine.check("power-dev", actions::NODE_ADMIN_ROTATE_KEYS),
            Decision::Allow
        );
        assert_eq!(
            engine.check("power-dev", actions::NODE_ADMIN_RESTART),
            Decision::Allow
        );
    }

    #[test]
    fn policy_json_round_trip() {
        let policy = make_policy();
        let bytes = serde_json::to_vec(&policy).unwrap();
        let parsed = Policy::from_json(&bytes).unwrap();
        assert_eq!(parsed.assignments.len(), 4);
        let engine = PolicyEngine::new(parsed);
        assert_eq!(engine.principal_count(), 4);
    }

    #[test]
    fn action_wildcard_patterns() {
        assert!(action_matches("*", "anything.at.all"));
        assert!(action_matches("rpc.read.*", "rpc.read.block"));
        assert!(action_matches("rpc.read.*", "rpc.read"));
        assert!(!action_matches("rpc.read.*", "rpc.write.block"));
        assert!(action_matches("contract.deploy", "contract.deploy"));
        assert!(!action_matches("contract.deploy", "contract.upgrade"));
    }

    #[test]
    fn check_and_audit_records_decision() {
        let engine = PolicyEngine::new(make_policy());
        let log = AuditLog::new(Box::new(MemorySink::new()));
        let decision = engine.check_and_audit("alice@karoowa", actions::NODE_ADMIN_RESTART, &log);
        assert_eq!(decision, Decision::Allow);
        let denied = engine.check_and_audit("monitor", actions::CONTRACT_DEPLOY, &log);
        assert_eq!(denied, Decision::Deny);
        // Both checks advance the log sequence.
        assert_eq!(log.next_sequence(), 2);
    }

    #[test]
    fn roles_of_returns_assigned_roles() {
        let engine = PolicyEngine::new(make_policy());
        let alice = engine.roles_of("alice@karoowa");
        assert_eq!(alice, vec![Role::Admin]);
        let missing = engine.roles_of("nobody");
        assert!(missing.is_empty());
    }
}
