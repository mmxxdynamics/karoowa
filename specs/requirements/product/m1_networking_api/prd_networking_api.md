# PRD: Networking & API Gateway (M1 Phases 1.5-1.6)

| Field | Value |
|-------|-------|
| Created | 2026-04-11 |
| Created By | Karoowa team (drafted by Claude) |
| Milestone | M1 (v0.1) — Foundation |
| Implementation Ticket | N/A — feature PRD covering multiple phases |
| Reviewers Requested | TBD |
| Reviewers | — |

> **Milestone:** 1 — Foundation (v0.1)
> **Feature:** Networking & API Gateway (Phases 1.5, 1.6)
> **Owner:** TBD
> **Stakeholders:** Core maintainers, chain builder teams, dApp developers, validator operators
> **Status:** Draft
> **Created:** 2026-04-11
> **Last Updated:** 2026-04-11
> **Parent PRD:** `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md`

---

## 1. Business Objective & Outcomes

### Business Objective

Deliver peer-to-peer networking and the unified API gateway so that Karoowa nodes can discover each other, broadcast blocks and transactions across a network, and serve external clients via JSON-RPC, REST, and WebSocket on a single port. After this ships, a multi-node network can reach consensus, and external applications can query chain state and submit transactions.

This is the third of six M1 feature PRDs. It depends on Feature PRD 1 (Foundation & Core) for types and Feature PRD 2 (Storage & Consensus) for the block production and persistence that networking distributes and the API exposes.

### Expected Business Outcomes

- **Nodes form a network.** Karoowa nodes discover each other via Kademlia, exchange blocks and transactions via Gossipsub, and maintain a connected peer mesh.
- **External clients have a single entry point.** JSON-RPC, REST, and WebSocket are served on one port via an Axum gateway, simplifying deployment and firewall configuration.
- **All 14 JSON-RPC methods work.** The `kw_*` method surface from the parent PRD is fully implemented for M1 (12 read methods + 2 write methods with placeholder mempool).
- **Observability is built in.** `/health` and `/metrics` endpoints ship with the API gateway, not as afterthoughts.

### Key Metrics

| Metric | Target | Current Baseline |
|--------|--------|-----------------|
| Peer discovery time (two nodes on same LAN) | < 10s | N/A |
| Block broadcast latency (2-node network) | < 1s | N/A |
| JSON-RPC `kw_getBalance` p99 latency (warm cache) | < 50ms | N/A |
| Connected peer count accuracy (`kw_peerCount` vs actual) | Exact | N/A |
| `/health` response time | < 10ms | N/A |

### User Problems

- **Isolated nodes are useless.** Without networking, each node is a standalone process that can't participate in a multi-validator network or share state.
- **No way for external apps to interact with the chain.** Without an API gateway, dApp developers have no interface to query state or submit transactions.
- **Multiple ports/protocols are an ops burden.** Operators shouldn't need to configure separate ports for RPC, REST, and WebSocket.

### Hypotheses / Problem Statements

| ID | Hypothesis | Metric | Validation |
|----|-----------|--------|------------|
| H-NA-001 | We believe that **libp2p Gossipsub + Kademlia** is sufficient for networks up to 1,000 nodes | Peer connectivity and message delivery rate at scale | Stress test on synthetic network during M2/M3 |
| H-NA-002 | We believe that **a single-port Axum gateway** serving JSON-RPC, REST, and WebSocket simplifies deployment without sacrificing performance | Deployment complexity (port count); p99 latency per protocol | Benchmark all three protocols under concurrent load |
| H-NA-003 | We believe that **built-in Prometheus metrics and health endpoints** reduce time-to-observability for operators from hours to minutes | Time from node start to Grafana dashboard showing live data | Measure during Phase 1.9 (Docker devnet) |

---

## 2. User Stories & User Flows

### User Stories

| ID | User Story | Spec Reference | Parent US |
|----|-----------|----------------|-----------|
| US-NA-001 | As a **Chain Builder**, I want nodes to discover each other and form a peer network, so that my multi-validator devnet can share blocks and transactions. | Phase 1.5 (T1.5.1-T1.5.8); parent REQ-001 | US-002 |
| US-NA-002 | As a **Chain Builder**, I want blocks and transactions broadcast across the network via Gossipsub, so that all validators see the same data. | Phase 1.5 (T1.5.4-T1.5.5); parent REQ-001 | US-002 |
| US-NA-003 | As a **dApp Developer**, I want JSON-RPC endpoints (`kw_*` methods) to query chain state and submit transactions, so that I can build applications against Karoowa. | Phase 1.6 (T1.6.2-T1.6.4); parent REQ-001, REQ-008 | US-005, US-006 |
| US-NA-004 | As a **dApp Developer**, I want REST endpoints mirroring the RPC surface, so that I can use whichever protocol fits my application. | Phase 1.6 (T1.6.5); parent REQ-008 | US-006 |
| US-NA-005 | As a **Validator Operator**, I want `/health` and `/metrics` endpoints, so that I can monitor node liveness and integrate with Prometheus/Grafana. | Phase 1.6 (T1.6.6); parent REQ-001 | US-008 |
| US-NA-006 | As a **dApp Developer**, I want a WebSocket endpoint for future subscription support, so that my application can prepare for real-time push notifications. | Phase 1.6 (T1.6.7); parent REQ-008 | US-006, US-007 |

### Primary Personas

| Persona | Relevance to this PRD |
|---------|----------------------|
| **Chain Builder** | Needs networking for multi-validator devnet; configures bootnodes and peer settings. |
| **dApp Developer** | Primary consumer of the API gateway — queries state, submits transactions, connects via WebSocket. |
| **Validator Operator** | Uses `/health` and `/metrics` for monitoring; cares about peer count and connection stability. |

### User Flows in Scope

| Flow | Description | Primary Persona |
|------|-------------|----------------|
| **Peer discovery** | Node A starts with bootnode list -> discovers Node B via Kademlia -> establishes connection -> both report each other in peer count | Chain Builder |
| **Block broadcast** | Node A produces a block -> broadcasts via Gossipsub -> Node B receives and validates -> both agree on chain head | Chain Builder |
| **JSON-RPC query** | Client sends `kw_getBalance` POST to `/rpc` -> gateway dispatches to handler -> returns JSON-RPC 2.0 response with balance | dApp Developer |
| **REST query** | Client sends GET to `/api/v1/blocks/5` -> gateway returns block at height 5 as JSON | dApp Developer |
| **Health check** | Operator probes `/health` -> receives HTTP 200 with node status -> scrapes `/metrics` for Prometheus | Validator Operator |
| **WebSocket handshake** | Client opens WebSocket to `/ws` -> handshake succeeds -> ping/pong works -> placeholder subscribe handler responds | dApp Developer |

---

## 3. High-Level Requirements

### Phase 1.5 — `karoowa-network` (libp2p)

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-NA-001 | libp2p transport stack: TCP + Noise + Yamux. Skeleton `Network` struct wrapping a libp2p `Swarm` | US-NA-001 | Must Have | See below |
| REQ-NA-002 | Identity: derive PeerId from Keypair (libp2p ed25519, distinct from validator keys) | US-NA-001 | Must Have | See below |
| REQ-NA-003 | Kademlia peer discovery: bootnode list config, peer discovery, peer routing | US-NA-001 | Must Have | See below |
| REQ-NA-004 | Gossipsub: topics for `blocks` and `transactions`, message validation hooks | US-NA-002 | Must Have | See below |
| REQ-NA-005 | Outbound API: `broadcast_block(block)`, `broadcast_transaction(tx)`. Inbound API: `subscribe_to_blocks()`, `subscribe_to_transactions()` returning async streams | US-NA-002 | Must Have | See below |
| REQ-NA-006 | Connection lifecycle: connect, disconnect, peer score, ban list | US-NA-001 | Must Have | See below |
| REQ-NA-007 | Expose current connected peer count for `kw_peerCount` | US-NA-001, US-NA-005 | Must Have | See below |
| REQ-NA-008 | Integration test: two in-process nodes, broadcast a block from one, verify the other receives it within 1 second | US-NA-002 | Must Have | See below |

### Phase 1.6 — `karoowa-api` (Axum gateway)

| ID | Requirement | User Story | Priority | BDD Scenarios |
|----|------------|-----------|----------|---------------|
| REQ-NA-009 | Axum router: `/rpc` (POST, JSON-RPC), `/api/v1/*` (REST), `/ws` (WebSocket upgrade), `/health`, `/metrics` | US-NA-003, US-NA-004, US-NA-005, US-NA-006 | Must Have | See below |
| REQ-NA-010 | JSON-RPC 2.0 dispatcher: parse request, route by method name, return response with proper error handling | US-NA-003 | Must Have | See below |
| REQ-NA-011 | 12 read methods: `kw_chainId`, `kw_blockNumber`, `kw_getBlockByNumber`, `kw_getBlockByHash`, `kw_getTransactionByHash`, `kw_getTransactionReceipt`, `kw_getBalance`, `kw_getTransactionCount`, `kw_getCode`, `kw_syncing`, `kw_peerCount`, `kw_nodeInfo` | US-NA-003 | Must Have | See below |
| REQ-NA-012 | 2 write methods: `kw_sendRawTransaction` (broadcast + add to placeholder pending pool), `kw_pendingTransactions` (read placeholder pending pool). Real mempool ships in M2. | US-NA-003 | Must Have | See below |
| REQ-NA-013 | REST equivalents: `/api/v1/status`, `/api/v1/blocks/<height>`, `/api/v1/blocks/<hash>`, `/api/v1/tx/<hash>`, etc. | US-NA-004 | Must Have | See below |
| REQ-NA-014 | `/health` returning HTTP 200 with basic node status. `/metrics` exposing Prometheus metrics (block height, peer count, RPC request count, RPC latency histograms) | US-NA-005 | Must Have | See below |
| REQ-NA-015 | WebSocket endpoint: handshake, ping/pong, placeholder subscribe handler. Real subscriptions ship in M2 Phase 2.1. | US-NA-006 | Should Have | See below |

### BDD Scenarios

#### REQ-NA-001: libp2p transport stack

**Scenario: Network struct initializes with TCP + Noise + Yamux**
**Given** a valid network configuration with a listen address
**When** the developer creates a `Network` instance
**Then** the underlying libp2p Swarm starts listening on the configured address
**And** the transport uses Noise for encryption and Yamux for multiplexing

**Sad Paths** *(to be added during refinement)*

#### REQ-NA-003: Kademlia peer discovery

**Scenario: Node discovers peers via bootnode**
**Given** Node A is running and listed as a bootnode
**And** Node B starts with Node A's address in its bootnode list
**When** Node B's Kademlia DHT bootstraps
**Then** Node B discovers Node A as a peer
**And** both nodes report each other in their peer count

**Sad Paths** *(to be added during refinement)*

#### REQ-NA-004: Gossipsub block broadcast

**Scenario: Block broadcast reaches all peers**
**Given** Node A and Node B are connected and subscribed to the `blocks` Gossipsub topic
**When** Node A broadcasts a block via `broadcast_block(block)`
**Then** Node B receives the block via its `subscribe_to_blocks()` stream within 1 second

**Sad Paths** *(to be added during refinement)*

#### REQ-NA-008: Two-node integration test

**Scenario: Two in-process nodes exchange a block**
**Given** two `Network` instances running in-process on different ports
**And** connected to each other via bootnode configuration
**When** Node A broadcasts a block
**Then** Node B receives the block within 1 second
**And** both nodes agree on the block hash

**Sad Paths** *(to be added during refinement)*

#### REQ-NA-009: Single-port multi-protocol gateway

**Scenario: All three API protocols are reachable on a single port**
**Given** a running Karoowa node bound to port 8545
**When** a client sends a JSON-RPC POST to `http://node:8545/rpc`
**Then** the client receives a valid JSON-RPC 2.0 response
**And** a REST GET to `http://node:8545/api/v1/status` returns HTTP 200 with node status
**And** a WebSocket upgrade to `ws://node:8545/ws` succeeds

**Sad Paths** *(to be added during refinement)*

#### REQ-NA-011: JSON-RPC read methods

**Scenario: kw_getBalance returns the correct balance**
**Given** a running node with an account at address `0xabc...` holding 1000 units
**When** a client sends `{"jsonrpc":"2.0","id":1,"method":"kw_getBalance","params":["0xabc..."]}`
**Then** the response contains `"result": "1000"`

**Scenario: kw_blockNumber returns the current chain height**
**Given** a running node that has produced 10 blocks
**When** a client sends `{"jsonrpc":"2.0","id":1,"method":"kw_blockNumber","params":[]}`
**Then** the response contains `"result": "10"`

**Scenario: kw_getBlockByHash returns the correct block**
**Given** a running node with a block with known hash `0xdef...`
**When** a client sends `{"jsonrpc":"2.0","id":1,"method":"kw_getBlockByHash","params":["0xdef..."]}`
**Then** the response contains the full block data with matching hash

**Sad Paths** *(to be added during refinement)*

#### REQ-NA-012: JSON-RPC write methods

**Scenario: kw_sendRawTransaction broadcasts and returns hash**
**Given** a running node connected to peers
**And** a valid signed transaction
**When** a client submits the transaction via `kw_sendRawTransaction`
**Then** the response contains the transaction hash
**And** the transaction appears in `kw_pendingTransactions`

**Sad Paths** *(to be added during refinement)*

#### REQ-NA-014: Health and metrics endpoints

**Scenario: Health endpoint reports node status**
**Given** a running Karoowa node
**When** a client sends GET to `/health`
**Then** the response is HTTP 200 with JSON containing node status (syncing, block height, peer count)

**Scenario: Metrics endpoint exposes Prometheus format**
**Given** a running Karoowa node that has produced blocks
**When** a client sends GET to `/metrics`
**Then** the response contains Prometheus-format metrics including `karoowa_block_height`, `karoowa_peer_count`, `karoowa_rpc_request_total`, and `karoowa_rpc_latency_seconds`

**Sad Paths** *(to be added during refinement)*

#### REQ-NA-015: WebSocket placeholder

**Scenario: WebSocket handshake succeeds**
**Given** a running Karoowa node
**When** a client opens a WebSocket connection to `ws://node:8545/ws`
**Then** the handshake succeeds
**And** ping/pong frames are exchanged
**And** sending a subscribe request returns a "not yet implemented" response with a clear message pointing to M2

**Sad Paths** *(to be added during refinement)*

---

## 4. Non-Functional Requirements

| ID | Category | Requirement | Target |
|----|----------|------------|--------|
| NFR-NA-001 | Performance | JSON-RPC `kw_getBalance` p99 latency (warm cache) | < 50ms |
| NFR-NA-002 | Performance | Block broadcast latency (2-node LAN) | < 1s |
| NFR-NA-003 | Scalability | Node supports at least 100 connected peers via libp2p | Validated on devnet stress test |
| NFR-NA-004 | Reliability | Node handles peer disconnect/reconnect without crashing | Verified by integration tests |
| NFR-NA-005 | Observability | `/metrics` includes block height, peer count, RPC throughput, RPC latency histograms | Verified by Prometheus scrape |

---

## 5. Assumptions

| ID | Assumption | Impact if Wrong | Validation Approach |
|----|-----------|----------------|-------------------|
| ASM-NA-001 | libp2p Gossipsub + Kademlia is sufficient for intended network sizes (inherits parent ASM-004) | Networking layer needs additional protocols for larger networks | Stress test at scale during M2/M3 |
| ASM-NA-002 | TCP + Noise + Yamux is the right transport stack. QUIC may be needed later but not for M1. | If QUIC is needed for NAT traversal, transport config changes | Monitor NAT issues during public devnet (Phase 1.10) |
| ASM-NA-003 | A placeholder pending pool is acceptable for M1. Real mempool ships in M2. | If dApp demos need mempool features (ordering, eviction), M2 work pulls forward | Confirm with sponsor that PoA + placeholder pool is sufficient for M1 demos |
| ASM-NA-004 | Axum is the right framework for the API gateway | Axum is well-maintained, async-first, and integrates well with tokio | Already validated by ecosystem adoption |

---

## 6. Dependencies & Exclusions

### Dependencies

| ID | Dependency | Owner | Status | Impact |
|----|-----------|-------|--------|--------|
| DEP-NA-001 | Feature PRD 1 (Foundation & Core) — types | Karoowa team | Pending | All types serialized/broadcast |
| DEP-NA-002 | Feature PRD 2 (Storage & Consensus) — block production and persistence | Karoowa team | Pending | Networking distributes consensus output; API reads from storage |
| DEP-NA-003 | `libp2p` crate (Rust) | Upstream | In Progress | P2P networking |
| DEP-NA-004 | `axum` + `tokio` | Upstream | Resolved | API gateway |
| DEP-NA-005 | `prometheus` / `metrics` crate | Upstream | Resolved | Metrics exposition |

### Exclusions

| Item | Rationale | Future Feature PRD |
|------|-----------|-------------------|
| Real mempool | M2 Phase 2.0 | M2 |
| WebSocket subscriptions (full) | M2 Phase 2.1 | M2 |
| Rate limiting / auth on API | Not needed for devnet; enterprise concern | Post-M1 |
| TLS termination | Handled by reverse proxy in production | Ops concern, not application |
| QUIC transport | TCP is sufficient for M1 | Post-M1 if needed |

---

## 7. Design Links

| Type | Link | Status |
|------|------|--------|
| Parent PRD | `specs/requirements/product/m0_karoowa_overview/prd_karoowa_overview.md` | Approved |
| Development plan | `specs/development/dev_plan.md` (Phases 1.5, 1.6) | Authoritative |
| Predecessor PRDs | Feature PRD 1 (Foundation & Core), Feature PRD 2 (Storage & Consensus) | Draft |

---

## 8. Open Questions

| ID | Question | Assignee | Due Date | Answer | Status |
|----|----------|----------|----------|--------|--------|
| OQ-NA-001 | Should PeerId be derived from the validator keypair or a separate networking keypair? dev_plan says "distinct from validator keys but can share entropy". Confirm. | Tech lead | Before T1.5.2 | — | Open |
| OQ-NA-002 | Should the API gateway enforce any request size limits in M1, or defer to M2 when mempool validation is in place? | Tech lead | Before T1.6.2 | — | Open |
| OQ-NA-003 | What Prometheus metric naming convention? `karoowa_*` prefix assumed. Confirm. | Tech lead | Before T1.6.6 | — | Open |
| OQ-NA-004 | Should the REST API be versioned (`/api/v1/`) from the start? dev_plan assumes yes. Confirm. | Tech lead | Before T1.6.5 | — | Open |

---

## 9. Out of Scope

| Item | Rationale | Future Milestone / Feature |
|------|-----------|---------------------------|
| Full WebSocket subscription manager | M2 Phase 2.1 | M2 |
| Mempool with ordering, eviction, replace-by-fee | M2 Phase 2.0 | M2 |
| NAT traversal / hole punching | TCP direct connection is sufficient for M1 devnets | Post-M1 |
| API authentication / authorization | Devnet is permissionless; enterprise auth is post-M1 | Enterprise layer |
| GraphQL API | Not in M1 scope | Post-M1 if needed |

---

## Changelog

| Date | Changes | Source |
|------|---------|--------|
| 2026-04-11 | Initial draft. Feature PRD covering M1 Phases 1.5-1.6. | Generated from `dev_plan.md` Phases 1.5-1.6 and parent PRD |
