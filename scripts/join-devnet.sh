#!/usr/bin/env bash
# Karoowa devnet — one-shot "join the public devnet" script for teammates.
#
# What it does:
#   1. Builds the `karoowa` binary in release mode (skipped if already built).
#   2. Generates a fresh validator key under ~/.karoowa/local.key (skipped if
#      one already exists).
#   3. Starts a local observer node and connects to the Lightsail bootnode.
#
# Usage:
#   bash scripts/join-devnet.sh                    # default p2p port 30303
#   P2P_PORT=30304 bash scripts/join-devnet.sh     # different port if 30303 is taken
#   RPC_PORT=8546  bash scripts/join-devnet.sh     # different RPC port
#
# After running, confirm peer link in another terminal:
#   curl -s http://localhost:${RPC_PORT:-8545}/health

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KEY_DIR="${KAROOWA_HOME:-$HOME/.karoowa}"
KEY_FILE="$KEY_DIR/local.key"
DATA_DIR="$KEY_DIR/data"

RPC_PORT="${RPC_PORT:-8545}"
P2P_PORT="${P2P_PORT:-30303}"

BOOTNODE="/ip4/18.171.150.73/tcp/30303/p2p/12D3KooWJitDPByarFixYTcXjPUYbagVYdG4Z3AuY3MY5KvDaJUD"
BIN="$REPO_ROOT/target/release/karoowa"

echo "==> Repository: $REPO_ROOT"
cd "$REPO_ROOT"

if [ ! -x "$BIN" ]; then
    echo "==> Building karoowa (release)..."
    cargo build --release --bin karoowa
fi

mkdir -p "$KEY_DIR"
if [ ! -f "$KEY_FILE" ]; then
    echo "==> Generating validator key..."
    "$BIN" wallet new --output "$KEY_FILE"
else
    echo "==> Reusing existing key: $KEY_FILE"
    "$BIN" wallet address "$KEY_FILE" || true
fi

echo "==> Starting node on RPC:$RPC_PORT / P2P:$P2P_PORT"
echo "    Bootnode: $BOOTNODE"
echo "    Data dir: $DATA_DIR"
echo
echo "    Verify peer link in another terminal:"
echo "        curl -s http://localhost:$RPC_PORT/health"
echo
exec "$BIN" node \
    --validator-key "$KEY_FILE" \
    --consensus poa \
    --data-dir "$DATA_DIR" \
    --bootnodes "$BOOTNODE" \
    --rpc-port "$RPC_PORT" \
    --p2p-port "$P2P_PORT" \
    --block-time 2
