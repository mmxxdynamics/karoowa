# M1 Foundation (v0.1) — Feature PRD Index

> **Status:** Superseded — M1 has been split into six feature PRDs.

The original M1 PRD was drafted on 2026-04-10 and superseded the same day after discovering that Karoowa is greenfield (not a rename of ChainFlux). On 2026-04-11, M1 was decomposed into six feature PRDs covering Phases 1.0 through 1.11.

## Feature PRDs

| # | Feature PRD | Phases | Scope |
|---|------------|--------|-------|
| 1 | [Foundation & Core](../m1_foundation_core/prd_foundation_core.md) | 1.0-1.2 | Workspace skeleton, CI, license stub, crypto primitives, core domain types |
| 2 | [Storage & Consensus](../m1_storage_consensus/prd_storage_consensus.md) | 1.3-1.4 | RocksDB storage traits + implementation, PoA consensus engine |
| 3 | [Networking & API](../m1_networking_api/prd_networking_api.md) | 1.5-1.6 | libp2p P2P networking, Axum API gateway (JSON-RPC, REST, WebSocket) |
| 4 | [Developer Tooling](../m1_developer_tooling/prd_developer_tooling.md) | 1.7-1.8 | Rust SDK, CLI binary (6 subcommands) |
| 5 | [Deployment & Install](../m1_deployment_install/prd_deployment_install.md) | 1.9-1.10 | Docker devnet, Grafana, hobbyist install, public devnet, faucet |
| 6 | [Agent Bundle](../m1_agent_bundle/prd_agent_bundle.md) | 1.11 | LLM provider trait, agent runtime, LanceDB memory, Onboarding/Monitoring/CLI agents |

## Related Documents

- **Parent PRD:** [`prd_karoowa_overview.md`](../m0_karoowa_overview/prd_karoowa_overview.md) — Overarching product vision (v0.1 → v1.0)
- **Development Plan:** [`dev_plan.md`](../../development/dev_plan.md) — Task-level breakdown (Phases 1.0 → 3.7)
