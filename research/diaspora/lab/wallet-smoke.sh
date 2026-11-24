#!/usr/bin/env bash
# Run only against locally built Bitcoin Core binaries. No public peers.
set -euo pipefail
bin=${1:?usage: bash wallet-smoke.sh /absolute/path/to/build/bin}
lab=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
data=$(mktemp -d)
port=${DIASPORA_RPC_PORT:-18443}
rpc() { "$bin/bitcoin-cli" -datadir="$data" -regtest -rpcport="$port" "$@"; }
wallet() { rpc -rpcwallet="$@"; }
cleanup() {
    rpc stop >/dev/null 2>&1 || true
    wait "$node" || true
    rm -rf -- "$data"
}
"$bin/bitcoind" -datadir="$data" -regtest -server -listen=0 -connect=0 \
    -dnsseed=0 -printtoconsole=0 -rpcbind=127.0.0.1 \
    -rpcport="$port" &
node=$!
trap cleanup EXIT
# A failed startup terminates rather than waiting forever for an RPC server.
for ((attempt=0; attempt<60; attempt++)); do
    kill -0 "$node"
    if rpc getblockchaininfo >/dev/null 2>&1; then break; fi
    sleep 1
done
rpc getblockchaininfo >/dev/null
rpc createwallet diaspora-lab >/dev/null
rpc createwallet receiver >/dev/null
mining_address=$(wallet diaspora-lab getnewaddress)
receive_address=$(wallet receiver getnewaddress)
rpc generatetoaddress 101 "$mining_address" >/dev/null
read_key() { python3 -c 'import json, sys; print(json.load(sys.stdin)[sys.argv[1]])' "$1"; }
outputs=$(printf '{"%s":1}' "$receive_address")
psbt=$(wallet diaspora-lab walletcreatefundedpsbt '[]' "$outputs" 0 '{"fee_rate":2}' | read_key psbt)
signed=$(wallet diaspora-lab walletprocesspsbt "$psbt" | read_key psbt)
rpc finalizepsbt "$signed" | read_key hex > "$data/tx.hex"
python3 "$lab/packet.py" encode "$data/tx.hex" "$data/packets"
python3 "$lab/packet.py" decode "$data/packets" "$data/restored.hex"
cmp "$data/tx.hex" "$data/restored.hex"
expected=$(rpc decoderawtransaction "$(cat "$data/tx.hex")" | read_key txid)
actual=$(rpc sendrawtransaction "$(cat "$data/restored.hex")")
test "$expected" = "$actual"
rpc generatetoaddress 1 "$mining_address" >/dev/null
wallet receiver getbalance | python3 -c 'from decimal import Decimal; import sys; assert Decimal(sys.stdin.read()) == 1'
echo "PASS: wallet creation, mature funds, transaction byte round trip and confirmed receipt"
