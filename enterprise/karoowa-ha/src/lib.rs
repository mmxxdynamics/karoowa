//! Karoowa Enterprise — high-availability node clustering.
//!
//! # Model
//!
//! Two Karoowa nodes race for a shared lease. The holder is the
//! **active** node — it produces blocks, answers RPC, holds the HSM
//! session. The loser is the **standby** — it syncs state from the
//! P2P network exactly like a validator observer, but its block
//! producer is suspended until it acquires the lease.
//!
//! Leader election is delegated to a pluggable [`LeaseBackend`].
//! Backends live out-of-process (SQL lease table, etcd, Consul, a
//! DynamoDB conditional write) so that a single node crash cannot
//! take both the lease and the backend down.
//!
//! # State machine
//!
//! ```text
//!   ┌──────────┐  acquire ok  ┌──────────┐  renew fails  ┌─────────┐
//!   │ Standby  │─────────────▶│  Active  │──────────────▶│ Failed  │
//!   └──────────┘              └──────────┘               └─────────┘
//!         ▲                         │                          │
//!         │      lease lost         │                          │
//!         └─────────────────────────┴──────────────────────────┘
//! ```
//!
//! A node starts in `Standby`, tries to acquire the lease on each
//! tick, and transitions to `Active` on success. While active it
//! renews every tick; a renewal failure (backend down, clock skew,
//! lease stolen) drops it back to `Standby`. `Failed` is an explicit
//! operator state for when the backend is unreachable past the
//! configured failure window — once there, the node stays down
//! until the operator restarts it.
//!
//! Side effects (starting the block producer, stopping RPC, pausing
//! the HSM session) are returned from [`HaCoordinator::tick`] as
//! [`StateTransition`] events so the node binary can act on them
//! outside this crate.

use std::sync::{Arc, Mutex};

use karoowa_audit_log::{AuditAction, AuditDraft, AuditLog};
use serde::{Deserialize, Serialize};

pub mod error;

pub use error::HaError;

/// Stable identifier for a node in the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        NodeId(id.into())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A single lease record. Backends return this as the canonical
/// representation of "who holds the lease right now".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub holder: NodeId,
    /// Unix seconds when the lease expires.
    pub expires_at: u64,
    /// Monotonic epoch — bumped on every successful handover. Used
    /// to fence stale writes: a node that lost the lease in epoch N
    /// cannot mutate state labelled with epoch N+1.
    pub epoch: u64,
}

/// Pluggable lease backend. Implementations live out-of-process and
/// must be correct under contention — the trait is the contract a
/// SQL / etcd / DynamoDB backend must satisfy.
pub trait LeaseBackend: Send + Sync {
    /// Return the current lease record, or `None` if no holder.
    fn read(&self) -> Result<Option<Lease>, HaError>;

    /// Attempt to acquire the lease for `node` until `expires_at`.
    /// Fails with `HaError::Contention` if another node already
    /// holds a non-expired lease.
    ///
    /// Implementations must perform this as a single atomic
    /// compare-and-swap (SQL: `INSERT … ON CONFLICT … WHERE
    /// expires_at < now()`; etcd: txn compare+put).
    fn acquire(&self, node: &NodeId, expires_at: u64, now: u64) -> Result<Lease, HaError>;

    /// Renew an existing lease. Fails with `HaError::LeaseLost`
    /// if the caller is not the current holder or the lease has
    /// already expired.
    fn renew(&self, node: &NodeId, expires_at: u64, now: u64) -> Result<Lease, HaError>;

    /// Explicitly release the lease. Used on graceful shutdown so a
    /// standby can take over immediately instead of waiting for
    /// expiry.
    fn release(&self, node: &NodeId) -> Result<(), HaError>;
}

/// Role the local node believes it's playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Holds the lease. Producing blocks.
    Active,
    /// Not the holder. Syncing via P2P, block producer suspended.
    Standby,
    /// Backend unreachable past the configured failure window —
    /// operator intervention required.
    Failed,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Active => f.write_str("active"),
            NodeState::Standby => f.write_str("standby"),
            NodeState::Failed => f.write_str("failed"),
        }
    }
}

/// What the coordinator needs the node binary to do after a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTransition {
    /// No change since the last tick.
    NoChange,
    /// Just became active — start block production, accept writes.
    BecameActive,
    /// Just stepped down — stop block production.
    BecameStandby,
    /// Backend failure exceeded the window. Node is now in `Failed`.
    BecameFailed,
}

/// Configuration for the coordinator.
#[derive(Debug, Clone)]
pub struct HaConfig {
    pub node_id: NodeId,
    /// How long each acquired lease lasts. Should be several ticks
    /// long so a single dropped packet doesn't cause a failover.
    /// 15s is a reasonable default for Postgres-backed deployments.
    pub lease_ttl_secs: u64,
    /// Consecutive backend errors before transitioning to `Failed`.
    pub failure_threshold: u32,
}

impl HaConfig {
    pub fn new(node_id: NodeId) -> Self {
        HaConfig {
            node_id,
            lease_ttl_secs: 15,
            failure_threshold: 5,
        }
    }
}

/// The coordinator. Owns the local state machine; the node binary
/// drives it by calling [`HaCoordinator::tick`] at a regular
/// cadence (typically every 5 seconds) and reacting to the returned
/// [`StateTransition`].
pub struct HaCoordinator {
    config: HaConfig,
    backend: Arc<dyn LeaseBackend>,
    inner: Mutex<CoordState>,
}

struct CoordState {
    state: NodeState,
    consecutive_errors: u32,
    last_epoch: Option<u64>,
}

impl HaCoordinator {
    pub fn new(config: HaConfig, backend: Arc<dyn LeaseBackend>) -> Self {
        HaCoordinator {
            config,
            backend,
            inner: Mutex::new(CoordState {
                state: NodeState::Standby,
                consecutive_errors: 0,
                last_epoch: None,
            }),
        }
    }

    /// Current believed state. Cheap, non-blocking.
    pub fn state(&self) -> NodeState {
        self.inner
            .lock()
            .map(|s| s.state)
            .unwrap_or(NodeState::Failed)
    }

    /// Epoch of the currently-held lease, if any.
    pub fn epoch(&self) -> Option<u64> {
        self.inner.lock().ok().and_then(|s| s.last_epoch)
    }

    /// Drive the state machine one step. The `now` arg is wall-clock
    /// in Unix seconds; the node binary injects it so tests can
    /// simulate time without tokio.
    pub fn tick(&self, now: u64) -> StateTransition {
        let result = self.tick_inner(now);
        let mut state = match self.inner.lock() {
            Ok(s) => s,
            Err(_) => return StateTransition::NoChange,
        };
        match result {
            Ok(new_state) => {
                state.consecutive_errors = 0;
                let old_state = state.state;
                state.state = new_state.0;
                if let Some(lease) = new_state.1 {
                    state.last_epoch = Some(lease.epoch);
                }
                if old_state == state.state {
                    StateTransition::NoChange
                } else {
                    match state.state {
                        NodeState::Active => StateTransition::BecameActive,
                        NodeState::Standby => StateTransition::BecameStandby,
                        NodeState::Failed => StateTransition::BecameFailed,
                    }
                }
            }
            Err(_) => {
                state.consecutive_errors += 1;
                if state.consecutive_errors >= self.config.failure_threshold {
                    let old = state.state;
                    state.state = NodeState::Failed;
                    if old != NodeState::Failed {
                        return StateTransition::BecameFailed;
                    }
                }
                StateTransition::NoChange
            }
        }
    }

    fn tick_inner(&self, now: u64) -> Result<(NodeState, Option<Lease>), HaError> {
        let expires_at = now + self.config.lease_ttl_secs;
        let current = self.backend.read()?;

        match current {
            // Someone holds a valid lease — might be us.
            Some(lease) if lease.expires_at > now => {
                if lease.holder == self.config.node_id {
                    // We're the holder → renew.
                    match self.backend.renew(&self.config.node_id, expires_at, now) {
                        Ok(l) => Ok((NodeState::Active, Some(l))),
                        Err(HaError::LeaseLost) => Ok((NodeState::Standby, None)),
                        Err(e) => Err(e),
                    }
                } else {
                    // Someone else is active — stay standby.
                    Ok((NodeState::Standby, None))
                }
            }
            // Lease vacant or expired — try to grab it.
            _ => match self.backend.acquire(&self.config.node_id, expires_at, now) {
                Ok(lease) => Ok((NodeState::Active, Some(lease))),
                Err(HaError::Contention) => Ok((NodeState::Standby, None)),
                Err(e) => Err(e),
            },
        }
    }

    /// Release the lease on graceful shutdown. Best-effort — backend
    /// errors are logged but don't block shutdown.
    pub fn shutdown(&self) {
        let _ = self.backend.release(&self.config.node_id);
        if let Ok(mut state) = self.inner.lock() {
            state.state = NodeState::Standby;
        }
    }

    /// Drive one tick and mirror the resulting transition to the
    /// audit log.
    pub fn tick_and_audit(&self, now: u64, audit: &AuditLog) -> StateTransition {
        let transition = self.tick(now);
        if !matches!(transition, StateTransition::NoChange) {
            let summary = format!(
                "ha.{}",
                match transition {
                    StateTransition::BecameActive => "became_active",
                    StateTransition::BecameStandby => "became_standby",
                    StateTransition::BecameFailed => "became_failed",
                    StateTransition::NoChange => unreachable!(),
                }
            );
            let draft = AuditDraft::new(
                AuditAction::AdminAuth,
                self.config.node_id.to_string(),
                summary,
            )
            .with_metadata(serde_json::json!({
                "ha_state": self.state().to_string(),
                "epoch": self.epoch(),
            }));
            let _ = audit.emit(draft);
        }
        transition
    }
}

// -----------------------------------------------------------------
// InMemoryLease — reference backend for tests & single-host dev
// -----------------------------------------------------------------

/// In-process lease backend. Useful for tests and single-host dev
/// clusters where both nodes share the same process (`cargo run`
/// dual-node harness). Not suitable for production — use a real
/// out-of-process backend (SQL, etcd) for actual HA.
#[derive(Default)]
pub struct InMemoryLease {
    inner: Mutex<Option<Lease>>,
}

impl InMemoryLease {
    pub fn new() -> Self {
        InMemoryLease::default()
    }
}

impl LeaseBackend for InMemoryLease {
    fn read(&self) -> Result<Option<Lease>, HaError> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| HaError::Internal("mutex poisoned".into()))?
            .clone())
    }

    fn acquire(&self, node: &NodeId, expires_at: u64, now: u64) -> Result<Lease, HaError> {
        let mut slot = self
            .inner
            .lock()
            .map_err(|_| HaError::Internal("mutex poisoned".into()))?;
        let next_epoch = match slot.as_ref() {
            Some(existing) if existing.expires_at > now => {
                return Err(HaError::Contention);
            }
            Some(existing) => existing.epoch + 1,
            None => 1,
        };
        let lease = Lease {
            holder: node.clone(),
            expires_at,
            epoch: next_epoch,
        };
        *slot = Some(lease.clone());
        Ok(lease)
    }

    fn renew(&self, node: &NodeId, expires_at: u64, now: u64) -> Result<Lease, HaError> {
        let mut slot = self
            .inner
            .lock()
            .map_err(|_| HaError::Internal("mutex poisoned".into()))?;
        let Some(existing) = slot.as_mut() else {
            return Err(HaError::LeaseLost);
        };
        if existing.holder != *node || existing.expires_at <= now {
            return Err(HaError::LeaseLost);
        }
        existing.expires_at = expires_at;
        Ok(existing.clone())
    }

    fn release(&self, node: &NodeId) -> Result<(), HaError> {
        let mut slot = self
            .inner
            .lock()
            .map_err(|_| HaError::Internal("mutex poisoned".into()))?;
        if slot.as_ref().map(|l| &l.holder) == Some(node) {
            *slot = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karoowa_audit_log::MemorySink;

    fn cfg(id: &str) -> HaConfig {
        HaConfig {
            node_id: NodeId::new(id),
            lease_ttl_secs: 10,
            failure_threshold: 3,
        }
    }

    #[test]
    fn first_tick_acquires_lease() {
        let backend = Arc::new(InMemoryLease::new());
        let coord = HaCoordinator::new(cfg("node-a"), backend.clone());
        let t = coord.tick(100);
        assert_eq!(t, StateTransition::BecameActive);
        assert_eq!(coord.state(), NodeState::Active);
        assert_eq!(coord.epoch(), Some(1));
        let lease = backend.read().unwrap().unwrap();
        assert_eq!(lease.holder, NodeId::new("node-a"));
    }

    #[test]
    fn second_node_stays_standby() {
        let backend = Arc::new(InMemoryLease::new());
        let a = HaCoordinator::new(cfg("node-a"), backend.clone());
        let b = HaCoordinator::new(cfg("node-b"), backend.clone());
        a.tick(100);
        let t = b.tick(101);
        assert_eq!(t, StateTransition::NoChange);
        assert_eq!(b.state(), NodeState::Standby);
    }

    #[test]
    fn active_node_renews_across_ticks() {
        let backend = Arc::new(InMemoryLease::new());
        let a = HaCoordinator::new(cfg("node-a"), backend.clone());
        a.tick(100);
        assert_eq!(a.tick(105), StateTransition::NoChange);
        assert_eq!(a.tick(108), StateTransition::NoChange);
        assert_eq!(a.state(), NodeState::Active);
        // Renewal extends expiry.
        let lease = backend.read().unwrap().unwrap();
        assert!(lease.expires_at > 108);
    }

    #[test]
    fn standby_takes_over_on_expiry() {
        let backend = Arc::new(InMemoryLease::new());
        let a = HaCoordinator::new(cfg("node-a"), backend.clone());
        let b = HaCoordinator::new(cfg("node-b"), backend.clone());
        a.tick(100);
        // Time advances past the lease TTL — node-a is gone.
        let t = b.tick(200);
        assert_eq!(t, StateTransition::BecameActive);
        assert_eq!(b.state(), NodeState::Active);
        assert_eq!(b.epoch(), Some(2));
    }

    #[test]
    fn graceful_shutdown_releases_lease() {
        let backend = Arc::new(InMemoryLease::new());
        let a = HaCoordinator::new(cfg("node-a"), backend.clone());
        let b = HaCoordinator::new(cfg("node-b"), backend.clone());
        a.tick(100);
        a.shutdown();
        assert!(backend.read().unwrap().is_none());
        // b can take over immediately — no need to wait for expiry.
        let t = b.tick(101);
        assert_eq!(t, StateTransition::BecameActive);
    }

    struct FailingBackend;
    impl LeaseBackend for FailingBackend {
        fn read(&self) -> Result<Option<Lease>, HaError> {
            Err(HaError::Unavailable("backend down".into()))
        }
        fn acquire(&self, _: &NodeId, _: u64, _: u64) -> Result<Lease, HaError> {
            Err(HaError::Unavailable("backend down".into()))
        }
        fn renew(&self, _: &NodeId, _: u64, _: u64) -> Result<Lease, HaError> {
            Err(HaError::Unavailable("backend down".into()))
        }
        fn release(&self, _: &NodeId) -> Result<(), HaError> {
            Err(HaError::Unavailable("backend down".into()))
        }
    }

    #[test]
    fn repeated_backend_errors_trigger_failed_state() {
        let backend: Arc<dyn LeaseBackend> = Arc::new(FailingBackend);
        let coord = HaCoordinator::new(cfg("node-a"), backend);
        // failure_threshold = 3 — first two ticks don't escalate.
        assert_eq!(coord.tick(1), StateTransition::NoChange);
        assert_eq!(coord.state(), NodeState::Standby);
        assert_eq!(coord.tick(2), StateTransition::NoChange);
        // Third tick crosses the threshold.
        assert_eq!(coord.tick(3), StateTransition::BecameFailed);
        assert_eq!(coord.state(), NodeState::Failed);
    }

    #[test]
    fn tick_and_audit_logs_transitions() {
        let backend = Arc::new(InMemoryLease::new());
        let coord = HaCoordinator::new(cfg("node-a"), backend);
        let log = AuditLog::new(Box::new(MemorySink::new()));
        coord.tick_and_audit(100, &log);
        // First tick: Standby → Active → one audit entry.
        assert_eq!(log.next_sequence(), 1);
        coord.tick_and_audit(105, &log);
        // Second tick: NoChange → no audit entry.
        assert_eq!(log.next_sequence(), 1);
    }

    #[test]
    fn epoch_increments_on_handover() {
        let backend = Arc::new(InMemoryLease::new());
        let a = HaCoordinator::new(cfg("node-a"), backend.clone());
        let b = HaCoordinator::new(cfg("node-b"), backend.clone());
        a.tick(100);
        assert_eq!(a.epoch(), Some(1));
        // A holds forever — b can't take over.
        b.tick(101);
        assert_eq!(b.epoch(), None);
        // A expires; b takes over.
        b.tick(200);
        assert_eq!(b.epoch(), Some(2));
    }
}
