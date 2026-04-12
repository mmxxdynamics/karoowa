# Karoowa Enterprise

This directory holds the **proprietary enterprise layer** of Karoowa. Code under this directory is **not** licensed under Apache 2.0 and is **not** distributed in community builds.

## Status

Phase 6.3 in progress. Shipped crates:

| Crate | Purpose | Status |
|---|---|---|
| `karoowa-license` | Signed license file parser; replaces `OssLicenseGate` at startup | ✅ shipped |
| `karoowa-audit-log` | Hash-chained append-only audit log (SOC 2 CC7.2) | ✅ shipped |
| `karoowa-rbac` | Role-based access control for node ops | ✅ shipped |
| `karoowa-hsm` | HSM integration (SoftHsm first; AWS CloudHSM/YubiHSM later) | ⏳ |
| `karoowa-ha` | Active/standby node clustering | ⏳ |
| `karoowa-marketplace` | Curated agent registry with attestation | ⏳ |

## What lives here (eventually)

Per parent PRD REQ-012:

- Agent governance / policy engine
- High-availability nodes
- Multi-tenancy
- Advanced analytics + GUI dashboards
- SSO / SAML / RBAC
- MPC key management
- Audit / compliance tooling
- M4 Governance + Finance/Treasury agents
- Custom SLAs and premium support tooling

## Rules

1. **Nothing in `core/` may import from `enterprise/`.** This is enforced by `scripts/check-cross-imports.sh` (Phase 1.0 task T1.0.4) running on every PR. Imports the other way (`enterprise/` → `core/`) are allowed.
2. **Community builds explicitly exclude this directory.** When the workspace eventually splits build profiles, the OSS profile will not list any `enterprise/*` paths in the workspace `members`.
3. **A signed license file is required** at startup for any enterprise feature. Modelled on Elasticsearch X-Pack. See parent PRD REQ-012 and decision D-012.
4. **The proprietary license** lives in the root `LICENSE-ENTERPRISE.md` file. It's a placeholder until an enterprise feature actually ships.

## See also

- `specs/strategy/02_technical_strategy.md` — Open-core strategy section
- `specs/strategy/03_decision_log.md` — D-011, D-012, D-013
- `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` — REQ-012
