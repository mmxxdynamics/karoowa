//! License gate for the Karoowa open-core boundary.
//!
//! The [`LicenseGate`] trait provides a runtime check for enterprise features.
//! The default [`OssLicenseGate`] always reports `Edition::Oss` with no
//! enterprise features enabled.
//!
//! Enterprise implementations (in the `enterprise/` directory) will provide
//! their own `LicenseGate` that parses a signed license file. That work is
//! deferred until M4 when the first enterprise feature ships.
//!
//! See decision D-012 in `specs/strategy/03_decision_log.md` and REQ-012 in
//! the parent PRD.

/// The edition of Karoowa this build is running as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Open-source community edition. No enterprise features available.
    Oss,
    /// Enterprise edition with a valid license.
    Enterprise,
}

impl std::fmt::Display for Edition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edition::Oss => write!(f, "oss"),
            Edition::Enterprise => write!(f, "enterprise"),
        }
    }
}

/// Information extracted from a license file (or defaulted for OSS builds).
#[derive(Debug, Clone)]
pub struct LicenseInfo {
    /// Which edition this license unlocks.
    pub edition: Edition,
    /// Names of enterprise features this license enables (empty for OSS).
    pub features: Vec<String>,
    /// Optional expiry timestamp (seconds since UNIX epoch). `None` = no expiry.
    pub expires_at: Option<u64>,
}

/// Trait for checking whether enterprise features are available at runtime.
///
/// The node binary calls this at startup to determine which capabilities to
/// enable. In M1, only [`OssLicenseGate`] exists. Enterprise implementations
/// will be added in `enterprise/` when the first enterprise feature ships.
pub trait LicenseGate: Send + Sync {
    /// Return the current license information.
    fn license_info(&self) -> LicenseInfo;

    /// Check whether a specific named enterprise feature is enabled.
    fn is_feature_enabled(&self, feature: &str) -> bool;
}

/// Default license gate for community / OSS builds.
///
/// Always returns [`Edition::Oss`] with no features enabled.
#[derive(Debug, Clone, Copy, Default)]
pub struct OssLicenseGate;

impl LicenseGate for OssLicenseGate {
    fn license_info(&self) -> LicenseInfo {
        LicenseInfo {
            edition: Edition::Oss,
            features: Vec::new(),
            expires_at: None,
        }
    }

    fn is_feature_enabled(&self, _feature: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oss_gate_reports_oss_edition() {
        let gate = OssLicenseGate;
        let info = gate.license_info();
        assert_eq!(info.edition, Edition::Oss);
        assert!(info.features.is_empty());
        assert!(info.expires_at.is_none());
    }

    #[test]
    fn oss_gate_denies_all_features() {
        let gate = OssLicenseGate;
        assert!(!gate.is_feature_enabled("multi-tenancy"));
        assert!(!gate.is_feature_enabled("rbac"));
        assert!(!gate.is_feature_enabled("anything"));
        assert!(!gate.is_feature_enabled(""));
    }

    #[test]
    fn edition_display() {
        assert_eq!(format!("{}", Edition::Oss), "oss");
        assert_eq!(format!("{}", Edition::Enterprise), "enterprise");
    }
}
