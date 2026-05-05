# Security Policy

The Karoowa team takes security seriously. This document explains how to
report vulnerabilities, what to expect from the response process, and which
versions of Karoowa receive security fixes.

## Reporting a vulnerability

**Do not open a public GitHub issue for a suspected vulnerability.**

Please report privately through one of these channels (in order of preference):

1. **GitHub Security Advisories** — open a private advisory at
   <https://github.com/mmxxdynamics/karoowa/security/advisories/new>.
   This is the preferred path because it lets us collaborate on a patch in a
   private fork.
2. **Email** — send the details to **security@karoowa.io**. PGP key:
   <https://karoowa.io/.well-known/security-pgp.asc>
   (fingerprint published on the team page once available).

Include, where possible:

- A clear description of the issue and its impact.
- Reproduction steps, proof-of-concept, or exploit code.
- The Karoowa version (commit hash or release tag) you tested against.
- Any suggested mitigation or fix.

## Response process

| Stage              | Target time                           |
| ------------------ | ------------------------------------- |
| Acknowledgement    | within 72 hours                       |
| Initial assessment | within 7 days                         |
| Fix or mitigation  | within 30 days for High/Critical      |
| Public disclosure  | coordinated, typically within 90 days |

We follow [coordinated disclosure](https://about.gitlab.com/handbook/security/disclosure/).
You will be credited in the advisory and changelog unless you ask otherwise.

## Severity

Severity is graded against the [CVSS 3.1](https://www.first.org/cvss/v3.1/specification-document)
scale, weighted by impact on:

- **Validator safety** — anything that could cause a chain split, halt, or
  invalid block acceptance.
- **Bridge safety** — issues that could lead to loss of bridged funds or
  forged cross-chain messages.
- **Operator safety** — issues that compromise key material, audit-log
  integrity, or RBAC controls in `enterprise/`.
- **Confidentiality / integrity** — leaked secrets, tampered storage.

## Supported versions

Karoowa is pre-1.0. Until v1.0, only the **latest minor release** receives
security fixes.

| Version           | Status                            |
| ----------------- | --------------------------------- |
| `0.6.x` (current) | Active                            |
| `0.5.x`           | Critical fixes for 30 days        |
| `< 0.5`           | Unsupported — please upgrade      |

After v1.0 we expect to support the latest two minor releases.

## Scope

In-scope:

- All code under `core/` and `enterprise/`.
- Default configurations shipped under `docker/`.
- Release artifacts produced by `.github/workflows/release.yml`.
- The install script at `scripts/install.sh` and the Homebrew tap.

Out of scope:

- Vulnerabilities that require physical access to a node.
- Issues in third-party services (RPC providers, block explorers).
- Self-XSS, missing security headers on docs sites, and SPF/DMARC records.
- Findings in unsupported versions.

## Bug bounty

A formal bounty programme will launch before the v1.0 mainnet cut. Until then,
we offer **public credit + Karoowa swag** for any disclosure that meets a
quality bar of "reproduces, has impact, suggests a mitigation."

## Hardening guides

- Operators: `docs/operator-guide.md`
- Validators: `docs/operator-guide.md#validator-hardening`
- HSM integration: `enterprise/karoowa-hsm/README.md`
