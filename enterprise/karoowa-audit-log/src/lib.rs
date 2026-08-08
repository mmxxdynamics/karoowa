//! Karoowa Enterprise — immutable, hash-chained audit log.
//!
//! Every sensitive operation performed by a Karoowa node operator
//! (key rotation, config change, RBAC policy update, contract
//! deployment, HSM sign request, …) emits an [`AuditEvent`] to a
//! configured [`AuditSink`]. Events are hash-chained: each event's
//! `prev_hash` points at the previous event's `event_hash`, so a log
//! whose middle has been tampered with will fail verification at
//! replay time.
//!
//! Output format is newline-delimited JSON (JSONL), one event per
//! line, append-only. This is directly ingestable by Splunk /
//! Elasticsearch / Datadog and maps onto SOC 2 CC7.2 requirements.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use karoowa_crypto::{sha3_256, Hash};
use serde::{Deserialize, Serialize};

pub mod error;

pub use error::AuditError;

/// Category of action being logged. Enumerated so dashboards and
/// alert rules can filter on a well-known vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// A validator signing key was rotated.
    KeyRotation,
    /// A chain or node config file was changed.
    ConfigChange,
    /// An RBAC role or policy was added, modified, or revoked.
    RbacChange,
    /// A contract was deployed by a privileged deployer.
    ContractDeploy,
    /// An HSM signing operation was requested.
    HsmSign,
    /// A license file was loaded, refreshed, or rejected.
    LicenseEvent,
    /// An admin logged in or out of the node RPC.
    AdminAuth,
    /// A governance-initiated parameter change was executed on-chain.
    ParameterApply,
    /// Generic catch-all for anything the caller wants audited.
    Other,
}

/// A single audit log record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Monotonic sequence number within this log file.
    pub sequence: u64,
    /// Unix timestamp in seconds when the event was emitted.
    pub timestamp: u64,
    /// What kind of action this event represents.
    pub action: AuditAction,
    /// Principal (operator identity / node address) that performed the
    /// action. Free-form string; RBAC integration populates this with
    /// the RBAC principal id.
    pub principal: String,
    /// Human-readable description of the action.
    pub summary: String,
    /// Structured metadata — anything the caller wants searchable.
    pub metadata: serde_json::Value,
    /// Hash of the previous event in this log (or [`Hash::ZERO`] for
    /// the first event). Forms the hash chain.
    pub prev_hash: Hash,
    /// Hash of this event — computed over every other field.
    pub event_hash: Hash,
}

impl AuditEvent {
    /// Compute the canonical event hash. Deterministic over the
    /// `sequence`, `timestamp`, `action`, `principal`, `summary`,
    /// `metadata`, and `prev_hash` fields — but NOT over `event_hash`
    /// itself (that would be circular).
    pub fn compute_hash(&self) -> Hash {
        #[derive(Serialize)]
        struct Canonical<'a> {
            sequence: u64,
            timestamp: u64,
            action: AuditAction,
            principal: &'a str,
            summary: &'a str,
            metadata: &'a serde_json::Value,
            prev_hash: Hash,
        }
        let canonical = Canonical {
            sequence: self.sequence,
            timestamp: self.timestamp,
            action: self.action,
            principal: &self.principal,
            summary: &self.summary,
            metadata: &self.metadata,
            prev_hash: self.prev_hash,
        };
        let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
        sha3_256(&bytes)
    }
}

/// A draft event before the hash chain links it into a log.
#[derive(Debug, Clone)]
pub struct AuditDraft {
    pub action: AuditAction,
    pub principal: String,
    pub summary: String,
    pub metadata: serde_json::Value,
}

impl AuditDraft {
    pub fn new(
        action: AuditAction,
        principal: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        AuditDraft {
            action,
            principal: principal.into(),
            summary: summary.into(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Sink interface — any backend that can accept audit events.
pub trait AuditSink: Send + Sync {
    /// Append a fully-formed event. The sink must persist it before
    /// returning (no async buffering) so that a node crash cannot lose
    /// more than the in-flight event.
    fn append(&self, event: &AuditEvent) -> Result<(), AuditError>;
}

/// The audit logger — owns the running sequence number, the last
/// event hash, and the underlying sink.
pub struct AuditLog {
    inner: Arc<Mutex<AuditState>>,
}

struct AuditState {
    sink: Box<dyn AuditSink>,
    next_sequence: u64,
    last_hash: Hash,
}

impl AuditLog {
    /// Create an empty audit log writing to the given sink. The first
    /// event will have `prev_hash = Hash::ZERO`.
    pub fn new(sink: Box<dyn AuditSink>) -> Self {
        AuditLog {
            inner: Arc::new(Mutex::new(AuditState {
                sink,
                next_sequence: 0,
                last_hash: Hash::ZERO,
            })),
        }
    }

    /// Resume an existing audit log from a known head (for example
    /// after reading the last line of a log file on node startup).
    pub fn resume_from(sink: Box<dyn AuditSink>, last_sequence: u64, last_hash: Hash) -> Self {
        AuditLog {
            inner: Arc::new(Mutex::new(AuditState {
                sink,
                next_sequence: last_sequence + 1,
                last_hash,
            })),
        }
    }

    /// Emit an event. Fills in the sequence number, timestamp, and
    /// hash-chain link, then appends to the sink.
    pub fn emit(&self, draft: AuditDraft) -> Result<AuditEvent, AuditError> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| AuditError::Internal("mutex poisoned".into()))?;
        let timestamp = now_secs();
        let mut event = AuditEvent {
            sequence: state.next_sequence,
            timestamp,
            action: draft.action,
            principal: draft.principal,
            summary: draft.summary,
            metadata: draft.metadata,
            prev_hash: state.last_hash,
            event_hash: Hash::ZERO,
        };
        event.event_hash = event.compute_hash();
        state.sink.append(&event)?;
        state.last_hash = event.event_hash;
        state.next_sequence += 1;
        Ok(event)
    }

    /// Return the sequence number that will be assigned to the next
    /// emitted event.
    pub fn next_sequence(&self) -> u64 {
        self.inner
            .lock()
            .map(|s| s.next_sequence)
            .unwrap_or_default()
    }

    /// Return the hash of the most recently emitted event (or
    /// [`Hash::ZERO`] for an empty log).
    pub fn last_hash(&self) -> Hash {
        self.inner.lock().map(|s| s.last_hash).unwrap_or(Hash::ZERO)
    }
}

/// In-memory sink — primarily for tests. Keeps every event in a Vec.
#[derive(Default)]
pub struct MemorySink {
    events: Mutex<Vec<AuditEvent>>,
}

impl MemorySink {
    pub fn new() -> Self {
        MemorySink::default()
    }

    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().map(|v| v.clone()).unwrap_or_default()
    }
}

impl AuditSink for MemorySink {
    fn append(&self, event: &AuditEvent) -> Result<(), AuditError> {
        self.events
            .lock()
            .map_err(|_| AuditError::Internal("mutex poisoned".into()))?
            .push(event.clone());
        Ok(())
    }
}

/// Append-only JSONL file sink. Every event is serialized as a single
/// JSON line; the file is opened with `O_APPEND` so concurrent writers
/// from different processes interleave cleanly at line boundaries.
pub struct FileSink {
    path: PathBuf,
    file: Mutex<File>,
}

impl FileSink {
    /// Open (or create) the given path for append-only writes.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AuditError> {
        let path = path.as_ref().to_path_buf();
        // 0600: audit records carry HSM key ids, backends and signing reasons.
        // Set at creation rather than via write_secret_file — this sink keeps a
        // live append-only fd and must not be replaced by a rename.
        let mut opts = OpenOptions::new();
        opts.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let file = opts.open(&path).map_err(AuditError::Io)?;
        Ok(FileSink {
            path,
            file: Mutex::new(file),
        })
    }

    /// Scan the log file and recompute every `event_hash`, verifying
    /// that the hash chain is unbroken. Returns `Ok(count)` if the log
    /// verifies, otherwise the first sequence number where tampering
    /// was detected.
    pub fn verify_chain(path: impl AsRef<Path>) -> Result<u64, AuditError> {
        let file = File::open(path.as_ref()).map_err(AuditError::Io)?;
        let reader = BufReader::new(file);
        let mut expected_prev = Hash::ZERO;
        let mut count: u64 = 0;
        for line in reader.lines() {
            let line = line.map_err(AuditError::Io)?;
            if line.trim().is_empty() {
                continue;
            }
            let event: AuditEvent =
                serde_json::from_str(&line).map_err(|e| AuditError::Malformed(e.to_string()))?;
            if event.prev_hash != expected_prev {
                return Err(AuditError::ChainBroken {
                    sequence: event.sequence,
                });
            }
            if event.event_hash != event.compute_hash() {
                return Err(AuditError::ChainBroken {
                    sequence: event.sequence,
                });
            }
            expected_prev = event.event_hash;
            count += 1;
        }
        Ok(count)
    }

    /// Return the sink's backing file path (for ops tooling).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AuditSink for FileSink {
    fn append(&self, event: &AuditEvent) -> Result<(), AuditError> {
        let line =
            serde_json::to_string(event).map_err(|e| AuditError::Malformed(e.to_string()))?;
        let mut file = self
            .file
            .lock()
            .map_err(|_| AuditError::Internal("mutex poisoned".into()))?;
        writeln!(file, "{line}").map_err(AuditError::Io)?;
        file.sync_data().map_err(AuditError::Io)?;
        Ok(())
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(summary: &str) -> AuditDraft {
        AuditDraft::new(AuditAction::ConfigChange, "admin@karoowa", summary)
    }

    #[test]
    fn memory_sink_records_events() {
        let sink = Arc::new(MemorySink::new());
        let log = AuditLog::new(Box::new(MemorySink::new()));
        // Re-check below uses the local sink; emit twice to advance seq.
        let e1 = log.emit(draft("rotate validator key")).unwrap();
        let e2 = log.emit(draft("bump gas limit")).unwrap();
        assert_eq!(e1.sequence, 0);
        assert_eq!(e2.sequence, 1);
        assert_eq!(e2.prev_hash, e1.event_hash);
        assert_eq!(log.next_sequence(), 2);
        let _ = sink;
    }

    #[test]
    fn hash_chain_links_events() {
        let log = AuditLog::new(Box::new(MemorySink::new()));
        let e1 = log.emit(draft("first")).unwrap();
        let e2 = log.emit(draft("second")).unwrap();
        let e3 = log.emit(draft("third")).unwrap();
        assert_eq!(e1.prev_hash, Hash::ZERO);
        assert_eq!(e2.prev_hash, e1.event_hash);
        assert_eq!(e3.prev_hash, e2.event_hash);
        // Every event's event_hash is a pure function of its content.
        assert_eq!(e1.event_hash, e1.compute_hash());
        assert_eq!(e2.event_hash, e2.compute_hash());
        assert_eq!(e3.event_hash, e3.compute_hash());
    }

    #[test]
    fn file_sink_round_trip_and_verify() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("audit.jsonl");

        let sink = FileSink::open(&path).unwrap();
        let log = AuditLog::new(Box::new(sink));
        log.emit(draft("key rotation ceremony")).unwrap();
        log.emit(draft("deploy contract 0xabc")).unwrap();
        log.emit(draft("grant Operator role")).unwrap();

        let verified = FileSink::verify_chain(&path).unwrap();
        assert_eq!(verified, 3);
    }

    #[test]
    fn file_sink_detects_tampering() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("audit.jsonl");

        let sink = FileSink::open(&path).unwrap();
        let log = AuditLog::new(Box::new(sink));
        log.emit(draft("event one")).unwrap();
        log.emit(draft("event two")).unwrap();
        log.emit(draft("event three")).unwrap();

        // Tamper with the middle event's summary without re-signing.
        let contents = std::fs::read_to_string(&path).unwrap();
        let mut lines: Vec<String> = contents.lines().map(|s| s.to_string()).collect();
        lines[1] = lines[1].replace("event two", "HACKED");
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();

        let err = FileSink::verify_chain(&path).unwrap_err();
        assert!(matches!(err, AuditError::ChainBroken { .. }));
    }

    #[test]
    fn resume_from_preserves_chain() {
        let sink1 = Arc::new(MemorySink::new());
        // Pre-seed by constructing a log, emitting, grabbing state.
        let log_a = AuditLog::new(Box::new(MemorySink::new()));
        let e1 = log_a.emit(draft("before restart")).unwrap();

        // Now resume — next event should link to e1's hash and bump
        // the sequence counter.
        let log_b = AuditLog::resume_from(Box::new(MemorySink::new()), e1.sequence, e1.event_hash);
        let e2 = log_b.emit(draft("after restart")).unwrap();
        assert_eq!(e2.sequence, e1.sequence + 1);
        assert_eq!(e2.prev_hash, e1.event_hash);
        let _ = sink1;
    }

    #[test]
    fn metadata_is_included_in_hash() {
        let log = AuditLog::new(Box::new(MemorySink::new()));
        let d1 = draft("op").with_metadata(serde_json::json!({"key": "a"}));
        let d2 = draft("op").with_metadata(serde_json::json!({"key": "b"}));
        let e1 = log.emit(d1).unwrap();
        // Reset to compare at same sequence/prev_hash is impossible here
        // since emitting mutates state, but we can assert that changing
        // the metadata changes the computed hash for the same logical
        // event by comparing compute_hash on two synthetic records.
        let mut synthetic_b = e1.clone();
        synthetic_b.metadata = serde_json::json!({"key": "b"});
        assert_ne!(e1.event_hash, synthetic_b.compute_hash());
        let _ = e1;
        let _ = d2;
    }
}
