//! Sidecar runtime mode.
//!
//! In sidecar mode, the agent runs as a separate process from the node.
//! All communication goes through a loopback-only HTTP proxy with:
//! - **Authentication:** local-only token verified on each request
//! - **Quota enforcement:** per-tool rate limits
//! - **Isolation:** agent process has no direct file/network access
//!   except via the proxy ("padded room" pattern)
//!
//! The sidecar is available from M2 and **mandatory from M3**.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the sidecar proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarConfig {
    /// Address the proxy listens on (loopback only).
    pub proxy_addr: String,
    /// The node's RPC URL that the proxy forwards to.
    pub node_rpc_url: String,
    /// Authentication token (generated at startup, shared with the agent).
    pub auth_token: String,
    /// Per-tool rate limits (tool_name → max calls per minute).
    pub rate_limits: HashMap<String, u32>,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        let token = format!("{:016x}", rand_token());
        let mut rate_limits = HashMap::new();
        rate_limits.insert("read_health".into(), 60);
        rate_limits.insert("read_metrics".into(), 30);
        rate_limits.insert("deploy_to_target".into(), 5);
        rate_limits.insert("rollback".into(), 5);

        SidecarConfig {
            proxy_addr: "127.0.0.1:9100".into(),
            node_rpc_url: "http://127.0.0.1:8545".into(),
            auth_token: token,
            rate_limits,
        }
    }
}

/// Agent runtime mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeMode {
    /// Agent runs inside the node process (M1 hobbyist default).
    InProcess,
    /// Agent runs as a separate process via loopback proxy (M2+, mandatory M3).
    Sidecar,
    /// Agent runs in Karoowa-managed cloud infra (enterprise only).
    CloudHosted,
}

impl std::fmt::Display for RuntimeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeMode::InProcess => write!(f, "in-process"),
            RuntimeMode::Sidecar => write!(f, "sidecar"),
            RuntimeMode::CloudHosted => write!(f, "cloud-hosted"),
        }
    }
}

impl std::str::FromStr for RuntimeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in-process" | "inprocess" => Ok(RuntimeMode::InProcess),
            "sidecar" => Ok(RuntimeMode::Sidecar),
            "cloud-hosted" | "cloud" => Ok(RuntimeMode::CloudHosted),
            _ => Err(format!(
                "unknown runtime mode: {s}. Available: in-process, sidecar, cloud-hosted"
            )),
        }
    }
}

/// Rate limiter tracking per-tool call counts.
#[derive(Debug)]
pub struct RateLimiter {
    limits: HashMap<String, u32>,
    counts: HashMap<String, (u32, std::time::Instant)>,
    window: std::time::Duration,
}

impl RateLimiter {
    /// Create a new rate limiter with per-minute windows.
    pub fn new(limits: HashMap<String, u32>) -> Self {
        RateLimiter {
            limits,
            counts: HashMap::new(),
            window: std::time::Duration::from_secs(60),
        }
    }

    /// Check if a tool call is allowed. Returns `true` if within limits.
    pub fn check(&mut self, tool_name: &str) -> bool {
        let limit = match self.limits.get(tool_name) {
            Some(&l) => l,
            None => return true, // No limit configured = unlimited.
        };

        let now = std::time::Instant::now();
        let entry = self.counts.entry(tool_name.to_string()).or_insert((0, now));

        // Reset window if expired.
        if now.duration_since(entry.1) > self.window {
            entry.0 = 0;
            entry.1 = now;
        }

        if entry.0 >= limit {
            return false;
        }

        entry.0 += 1;
        true
    }
}

/// Generate a random-ish token from system time (not cryptographically secure
/// — fine for local-only sidecar auth).
fn rand_token() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_mode_parse() {
        assert_eq!(
            "in-process".parse::<RuntimeMode>().unwrap(),
            RuntimeMode::InProcess
        );
        assert_eq!(
            "sidecar".parse::<RuntimeMode>().unwrap(),
            RuntimeMode::Sidecar
        );
        assert_eq!(
            "cloud-hosted".parse::<RuntimeMode>().unwrap(),
            RuntimeMode::CloudHosted
        );
        assert!("invalid".parse::<RuntimeMode>().is_err());
    }

    #[test]
    fn runtime_mode_display() {
        assert_eq!(RuntimeMode::InProcess.to_string(), "in-process");
        assert_eq!(RuntimeMode::Sidecar.to_string(), "sidecar");
    }

    #[test]
    fn rate_limiter_allows_within_limit() {
        let mut limits = HashMap::new();
        limits.insert("test_tool".into(), 3);
        let mut limiter = RateLimiter::new(limits);

        assert!(limiter.check("test_tool"));
        assert!(limiter.check("test_tool"));
        assert!(limiter.check("test_tool"));
        assert!(!limiter.check("test_tool")); // 4th call blocked
    }

    #[test]
    fn rate_limiter_allows_unlisted_tools() {
        let limiter_limits = HashMap::new();
        let mut limiter = RateLimiter::new(limiter_limits);
        assert!(limiter.check("any_tool")); // no limit = allowed
    }

    #[test]
    fn default_sidecar_config() {
        let config = SidecarConfig::default();
        assert_eq!(config.proxy_addr, "127.0.0.1:9100");
        assert!(!config.auth_token.is_empty());
        assert!(config.rate_limits.contains_key("read_health"));
    }
}
