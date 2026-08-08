# Karoowa: Operator Guide

**Audience:** ops engineers running a Karoowa node in production (validator, full node, or enterprise deployment).
**Target version:** v1.0 (in progress; latest tag is `v0.5.0`, though no release assets have been published yet — see #41).
**Last updated:** 2026-08-07.

> **Status note.** Capabilities marked _v1.0_ (HSM, HA lease backends,
> hard-upgrade migration framework, k8s reference manifests) describe the
> v1.0 surface area. Hardware sizing, RPC ports, monitoring metrics, and
> the systemd unit work today against `v0.5.0`. Authoritative reference
> is always `karoowa <cmd> --help`.

---

## 1. Hardware Requirements

| Role | CPU | RAM | Disk | Network |
|---|---|---|---|---|
| Validator (PoS / BFT) | 8 modern cores | 32 GB | 1 TB NVMe SSD | 1 Gbps, ≤50 ms p99 to peer set |
| Full node (archive) | 8 cores | 32 GB | 4 TB NVMe SSD | 1 Gbps |
| Full node (pruned) | 4 cores | 16 GB | 500 GB NVMe SSD | 100 Mbps |
| Light client | 2 cores | 4 GB | 20 GB SSD | 25 Mbps |

**RocksDB loves NVMe.** Spinning disks or network-attached storage (EBS gp2/gp3 without provisioned IOPS) will fall behind on busy networks. Use local NVMe or EBS `io2` with ≥3000 provisioned IOPS.

**System tuning:**
- `ulimit -n 65536` or higher (libp2p maintains many sockets)
- Enable `vm.overcommit_memory = 1` for RocksDB allocator behavior
- Disable swap on validator hosts to avoid GC pauses

---

## 2. Installation

### 2.1 From a release binary (recommended)

```bash
# One-liner installer (verifies SHA-256 checksum)
curl -fsSL https://install.karoowa.io | bash

# Or pin to a specific tag
VERSION=v0.6.0
curl -fsSL https://github.com/mmxxdynamics/karoowa/releases/download/${VERSION}/karoowa-${VERSION}-x86_64-unknown-linux-gnu.tar.gz \
  | tar xz -C /usr/local/bin
karoowa --version
```

Published Linux targets are `x86_64-unknown-linux-gnu` and
`aarch64-unknown-linux-gnu`, plus macOS (both architectures) and Windows.
The Linux tarballs are built against **glibc 2.39** (Ubuntu 24.04), so they will
not run on Debian 12, RHEL 9 or Ubuntu 22.04 — use §2.3 or build from source on
those.

> **No musl tarball today.** A statically-linked musl binary is not published.
> Beyond that build never having worked, musl is a poor fit for this workload:
> its default thread stack is 128 KB (RocksDB creates its compaction threads
> from C++, so they inherit that rather than Rust's 2 MiB), and its allocator
> benchmarks 5-10x slower than glibc under the multi-threaded, allocation-heavy
> access pattern RocksDB generates.
>
> The container image (§2.3) is **not** affected — it is built on Debian 12 and
> links glibc, matching its `distroless/cc-debian12` runtime. It previously
> built musl-static on Alpine, which could not work at all: `librocksdb-sys`
> runs bindgen, bindgen `dlopen`s libclang, and a statically linked build script
> cannot `dlopen`. Reintroducing a musl *tarball* is tracked in
> [#41](https://github.com/mmxxdynamics/karoowa/issues/41).

Verify the Sigstore keyless signature before running in production:

```bash
gh attestation verify karoowa-${VERSION}-x86_64-unknown-linux-gnu.tar.gz \
    --repo mmxxdynamics/karoowa
```

### 2.2 From source

```bash
git clone https://github.com/mmxxdynamics/karoowa
cd karoowa
cargo build --release --bin karoowa
install target/release/karoowa /usr/local/bin/
```

The MSRV is **Rust 1.94** (pinned in `rust-toolchain.toml`). `rustup` will
pick it up automatically.

### 2.3 Via Docker

```bash
docker run --rm -it \
  -v /var/lib/karoowa:/data \
  -p 8545:8545 \
  -p 30303:30303 \
  ghcr.io/mmxxdynamics/karoowa:0.6.0 \
  node \
      --validator-key /data/validator.key \
      --consensus poa \
      --data-dir /data \
      --rpc-port 8545 \
      --p2p-port 30303
```

> **The RPC is unauthenticated and binds `0.0.0.0` by default.** Only publish
> `8545` on a trusted network.
>
> On a bare-metal or systemd host that does not need remote RPC, pass
> `--rpc-bind 127.0.0.1`. **Do not do this inside a container** — a loopback
> bind is unreachable through a published port, so `-p 8545:8545` would refuse
> connections. Restrict container RPC at the network layer instead.
>
> Key files are `0600` and owned by the generating user; the image runs as
> `nonroot` (uid 65532), so a bind-mounted key must be readable by that uid.

---

## 3. First-time Setup

### 3.1 Generate a validator keypair

```bash
karoowa wallet new --output /var/lib/karoowa/keys/validator.key
```

Back up the output file **immediately** and store a copy in cold storage. Losing this key means losing your validator slot. For mainnet, generate keys inside an HSM (see §7).

> **The key file is written `0600`, owned by whoever ran the command.** If you
> generate it as `root` but run the service as `User=karoowa` (§4.1), the node
> cannot read it. Either generate it as the service user, or hand it over:
>
> ```bash
> chown karoowa:karoowa /var/lib/karoowa/keys/validator.key
> ```
>
> The same applies to containers — see §2.3.

### 3.2 Join a network

```bash
# Devnet (today)
karoowa node \
  --validator-key /var/lib/karoowa/keys/validator.key \
  --consensus poa \
  --data-dir /var/lib/karoowa/data \
  --rpc-port 8545 \
  --p2p-port 30303 \
  --bootnodes /ip4/bootnode-1.karoowa.io/tcp/30303/p2p/12D3KooW...
```

Or use the convenience script for the public devnet:

```bash
KAROOWA_BOOTNODE=/ip4/.../tcp/30303/p2p/... bash scripts/join-devnet.sh
```

> **v1.0:** named-network join (`--join testnet`) and per-network signed
> genesis files in `specs/genesis/` ship with v1.0. Verify the genesis
> hash against the value published on <https://karoowa.io/genesis>.

### 3.3 Initial sync

A fresh node does a state-sync from a recent snapshot rather than replaying every block from genesis. On a modern NVMe box testnet catches up in 10-30 minutes; archive sync from genesis is roughly 4-8 hours.

Watch the sync make progress with:

```bash
karoowa client status --rpc http://localhost:8545
```

---

## 4. Running the Node

The `karoowa` binary has seven subcommands:

| Subcommand | Purpose |
|---|---|
| `node` | Start the node (validator / full / light) |
| `wallet` | Generate, inspect, sign with keys |
| `genesis` | Build or validate a genesis config |
| `client` | One-shot RPC queries |
| `devnet` | Local devnet management |
| `network` | Peer inspection (`peers list`, `peers add`) |
| `agent` | Run an AI agent (onboard, monitor, dev) |

Run `karoowa <cmd> --help` for the full flag list.

### 4.1 systemd unit

```ini
# /etc/systemd/system/karoowa.service
[Unit]
Description=Karoowa node
After=network-online.target
Wants=network-online.target

[Service]
User=karoowa
ExecStart=/usr/local/bin/karoowa node \
    --validator-key /var/lib/karoowa/keys/validator.key \
    --consensus poa \
    --data-dir /var/lib/karoowa/data \
    --rpc-port 8545 \
    --p2p-port 30303
Restart=on-failure
RestartSec=5
LimitNOFILE=65536
KillSignal=SIGINT
TimeoutStopSec=60

[Install]
WantedBy=multi-user.target
```

`SIGINT` gives the node time to flush RocksDB and release the HA lease cleanly. A 60-second stop timeout covers the normal shutdown path.

**Harden the RPC on a validator.** The RPC binds `0.0.0.0` by default and has no
authentication of its own, so it exposes node control and mempool submission to
anything that can reach the host. Unless you deliberately serve remote clients,
add `--rpc-bind 127.0.0.1` to `ExecStart`, or firewall port 8545. The node logs
a warning at startup whenever it is bound to a non-loopback address.

### 4.2 Kubernetes

A minimal `StatefulSet`:

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: karoowa-validator
spec:
  serviceName: karoowa
  replicas: 1
  template:
    spec:
      securityContext:
        # The image already runs as 65532; stated here so the manifest does not
        # depend on that. fsGroup makes volume contents group-readable by 65532
        # — needed only if the key lives on the PVC (see the note below).
        # OnRootMismatch avoids a recursive chown of the whole 1Ti volume on
        # every pod start.
        runAsUser: 65532
        runAsGroup: 65532
        fsGroup: 65532
        fsGroupChangePolicy: OnRootMismatch
      terminationGracePeriodSeconds: 90
      containers:
        - name: karoowa
          image: ghcr.io/mmxxdynamics/karoowa:0.6.0
          args:
            - node
            - --validator-key
            - /data/validator.key
            - --consensus
            - poa
            - --data-dir
            - /data
            - --rpc-port
            - "8545"
            - --p2p-port
            - "30303"
          ports:
            - { name: rpc,  containerPort: 8545 }
            - { name: p2p,  containerPort: 30303 }
          # The image carries no Docker HEALTHCHECK — it is distroless, so there
          # is nothing in it to self-probe with, and the kubelet would ignore one
          # anyway. Probe the endpoints directly (see §5.3).
          readinessProbe:
            httpGet: { path: /ready, port: rpc }
            initialDelaySeconds: 15
            periodSeconds: 10
          livenessProbe:
            httpGet: { path: /health, port: rpc }
            initialDelaySeconds: 60
            periodSeconds: 30
            failureThreshold: 3
          volumeMounts:
            - { name: data, mountPath: /data }
          resources:
            requests: { cpu: "4", memory: "16Gi" }
            limits:   { cpu: "8", memory: "32Gi" }
  volumeClaimTemplates:
    - metadata: { name: data }
      spec:
        accessModes: [ "ReadWriteOnce" ]
        storageClassName: fast-nvme
        resources: { requests: { storage: 1Ti } }
```

> **Getting the key onto the pod.** The validator key is `0600` and the image
> runs as uid 65532, so how the key arrives matters.
>
> **Use a `Secret`** mounted read-only with `defaultMode: 0400`. The kubelet
> applies ownership when it sets the volume up, so this works without relying
> on `fsGroup` and without touching the data volume.
>
> If you instead place the key on the PVC, note that `fsGroup` is applied at
> **volume setup time**, before any container runs — it does not fix a key
> written afterwards by a root `initContainer`, which stays `root:root` and is
> unreadable at `0600`. In that case have the initContainer `chown 65532:65532`
> the key itself. See §3.1.

---

## 5. Monitoring

### 5.1 Prometheus endpoint

The node exposes `/metrics` on port `8545` (share with RPC) with standard Prometheus text format. Key metrics:

| Metric | Type | Alert threshold |
|---|---|---|
| `karoowa_consensus_height` | gauge | stalled > 30 s |
| `karoowa_consensus_is_leader` | gauge (0/1) |  |
| `karoowa_mempool_pending_txs` | gauge | > 10 000 |
| `karoowa_p2p_peer_count` | gauge | < 8 |
| `karoowa_block_production_duration_seconds` | histogram | p99 > 800 ms |
| `karoowa_vm_execution_fuel_used_total` | counter |  |
| `karoowa_storage_rocksdb_compaction_pending_bytes` | gauge | > 10 GB |
| `karoowa_governance_pending_proposals` | gauge |  |

### 5.2 Log structure

All logs go to stderr in `tracing_subscriber` format. Typical lines:

```
2026-04-12T10:23:14.123Z  INFO karoowa::consensus: block produced height=12345 txs=42 gas=15000000
2026-04-12T10:23:16.218Z  WARN karoowa::network: peer dropped peer=12D3Koo... reason="idle timeout"
```

For structured logs, set `KAROOWA_LOG_FORMAT=json`.

### 5.3 Health checks

- `GET /health`: returns 200 if the node is running, 503 if shutting down.
- `GET /ready`: returns 200 if synced (head within 2 blocks of peers), 503 otherwise. Use as the Kubernetes readiness probe.

---

## 6. Upgrades

### 6.1 Soft upgrades (non-breaking)

For a patch release that doesn't change the state format or consensus rules:

```bash
systemctl stop karoowa
install /tmp/karoowa /usr/local/bin/
systemctl start karoowa
```

The node resumes from the last persisted block.

**Downgrade floor: RocksDB 8.6.0.** Karoowa 0.6.0-dev onward embeds RocksDB 10.4.2,
which writes SST files at `format_version=6`. Older engines cannot read them, so
rolling back to a binary built against RocksDB earlier than 8.6.0 will fail to open
an existing data directory. Every Karoowa binary ever shipped embeds 8.10.0 or
newer, so rollback to any prior release is safe — but this is a one-way door if
the pin is ever moved backwards.

Note the asymmetry: upgrading is unconditionally safe (the format version is read
per-file from the SST footer, so pre-existing files stay readable, and the data
directory migrates to `format_version=6` gradually through normal compaction).
It is only *downgrading past 8.6.0* that is blocked.

### 6.2 Hard upgrades (breaking)

For a release with a state-format migration or consensus-breaking change:

1. Verify the upgrade-height announcement on <https://karoowa.io/upgrades>.
2. Download and extract the new binary **before** the upgrade height.
3. At the upgrade height the old binary will halt with `UpgradeRequired` in the logs.
4. Swap the binary and restart. The new binary runs any required migrations during startup.

Watch the upgrade chat channel on Discord for live ops coordination. Most validators cut over within 15 minutes of the halt.

---

## 7. HSM Integration (Enterprise)

Production validators should never hold their signing key in a file. Use an HSM.

### 7.1 SoftHsm (development / CI only): _v1.0_

```bash
karoowa node \
  --hsm-provider softhsm \
  --hsm-store /etc/karoowa/softhsm.json
```

**Not for mainnet.** SoftHsm keeps the key material in a JSON file; if the
host is compromised the key is compromised.

### 7.2 Real HSMs: _v1.1_

AWS CloudHSM and YubiHSM 2 drivers ship in v1.1. The `HsmProvider` trait
in `enterprise/karoowa-hsm/src/provider.rs` is stable today: integrations
implement the same trait that SoftHsm does.

### 7.3 Key rotation

> **Rotating a *file-based* key changes its owner.** This applies to
> `validator.key` (§3.1) and to `softhsm.json`, not to the HSM key ids below.
> Secret files are written by creating a new file and renaming it over the old
> one, so the result is owned by whoever ran the command — not by the previous
> owner. If you rotate as `root` a file that `User=karoowa` reads,
> `chown karoowa:karoowa` it again afterwards or the node will not restart.

1. Generate the new key in the HSM: `karoowa wallet hsm-generate --key-id validator-2`.
2. Submit a validator-set change on-chain via governance.
3. Wait for the change to finalize (2 epochs).
4. Restart the node with `--validator-key validator-2`.
5. Deactivate `validator-1` in the HSM.

A key rotation emits an `AuditAction::KeyRotation` event to the SOC 2 audit log.

---

## 8. Backup & Restore

> **Key files are `0600` and owned by the user that created them.** Use an
> archiver that preserves modes (`tar -p`, `rsync -a`) so a restored key is not
> silently widened — and after restoring as `root`, `chown` it back to the
> service user or the node will not be able to read it.

### 8.1 What to back up

- `$HOME/keys/validator.json` (or HSM key slot metadata)
- `$HOME/chain/rocksdb/` (full state: back up at restart, not hot)
- `$HOME/config.toml` + `$HOME/genesis.json`
- The SOC 2 audit log at `$HOME/audit.jsonl`: **never truncated, append-only**

### 8.2 Restore procedure

```bash
systemctl stop karoowa
rm -rf /var/lib/karoowa/chain
tar xzpf backup.tar.gz -C /var/lib/karoowa/
systemctl start karoowa
```

The node will resume from the backed-up height and catch up via state-sync.

---

## 9. HA / Active-Standby (Enterprise): _v1.0_

The `enterprise/karoowa-ha` crate ships lease-based active-standby for
single-host-pair deployments. The runtime CLI integration lands with v1.0:

```bash
# v1.0:
karoowa node \
  --ha-enabled \
  --ha-node-id validator-a \
  --ha-lease-backend inmemory   # v1.0
  # --ha-lease-backend etcd     # v1.1
```

Both nodes run the same config with different `--ha-node-id`. Only the
lease holder produces blocks; the standby hot-syncs via P2P and takes
over on expiry. See `enterprise/karoowa-ha/README.md` for the failover
SLAs and the lease-backend trait surface (stable today).

---

## 10. Runbooks (Incident Response)

### 10.1 Chain halt

- **Symptom:** `karoowa_consensus_height` flat for > 30 s across ≥ 1/3 of validators.
- **First check:** peer count and BFT round number. If `karoowa_bft_round` is climbing without commits, it's a liveness failure.
- **Action:** coordinate on Discord `#ops-incidents`. Do **not** restart unilaterally: a single validator rejoining can make a partition permanent.

### 10.2 Consensus split (two heights finalized)

- **Symptom:** two validators report different `head_hash` at the same height.
- **Action:** this should be cryptographically impossible under BFT. File an incident report with both head hashes and the validator set at that height. The incident response team triggers the external auditor hotline.

### 10.3 Key compromise

- **Symptom:** Suspected exfiltration of `validator.json` or HSM key slot.
- **Action:**
  1. Immediately execute a key rotation (see §7.3).
  2. Deactivate the old key in the HSM.
  3. Submit an `AuditAction::KeyRotation` report to `security@karoowa.io` with the rotation timeline.

### 10.4 Storage full

- **Symptom:** `karoowa_storage_rocksdb_compaction_pending_bytes` increasing; node cannot write.
- **Action:** stop the node, prune old blocks (`karoowa wallet prune --keep 100000`), restart. Archive nodes must expand the volume instead.

---

## 11. Support

- **Community:** <https://discord.gg/karoowa> `#ops` channel
- **Enterprise SLA:** `support@karoowa.io` (24×7 with a valid license)
- **Security incidents:** `security@karoowa.io` (PGP key in `SECURITY.md`)
- **Source:** <https://github.com/mmxxdynamics/karoowa>
