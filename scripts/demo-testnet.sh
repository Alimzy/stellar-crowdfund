#!/usr/bin/env bash
# End-to-end demo on Stellar testnet with native XLM. Takes about two minutes
# because it waits for real deadlines to pass.
#
#   Campaign 0: goal 100 XLM. Alice pledges 60, Bob pledges 70. Creator withdraws 130.
#   Campaign 1: goal 10,000 XLM. Bob pledges 50. Goal missed, Bob refunds 50.
#
# Writes docs/testnet-proof.md and fills the README proof section.
# Requires: stellar CLI, the wasm32v1-none Rust target, python3, network access.
set -euo pipefail
cd "$(dirname "$0")/.."

NET=testnet
CREATOR=crowdfund-creator
ALICE=crowdfund-alice
BOB=crowdfund-bob
PROOF=docs/testnet-proof.md
WAIT_SECONDS=90

need() { command -v "$1" >/dev/null || { echo "missing dependency: $1" >&2; exit 1; }; }
need stellar
need python3

tx_of() { grep -oE 'tx/[0-9a-f]{64}' | head -1 | cut -d/ -f2 || true; }
result_of() { grep -oE '"[0-9]+"' | tail -1 | tr -d '"' || true; }
xlm() { awk -v s="$1" 'BEGIN { printf "%.7g", s / 10000000 }'; }

echo "==> Build"
stellar contract build
WASM=target/wasm32v1-none/release/crowdfund.wasm
[ -f "$WASM" ] || WASM=$(find target -name crowdfund.wasm -path '*release*' | head -1)
WASM_HASH=$(sha256sum "$WASM" | cut -d' ' -f1)

echo "==> Identities"
for k in "$CREATOR" "$ALICE" "$BOB"; do
  stellar keys address "$k" >/dev/null 2>&1 || stellar keys generate "$k" --network "$NET" --fund
done
CREATOR_ADDR=$(stellar keys address "$CREATOR")
ALICE_ADDR=$(stellar keys address "$ALICE")
BOB_ADDR=$(stellar keys address "$BOB")

echo "==> Deploy"
DEPLOY_LOG=$(stellar contract deploy --wasm "$WASM" --source "$CREATOR" --network "$NET" 2>&1 | tee /dev/stderr)
CONTRACT=$(echo "$DEPLOY_LOG" | grep -oE 'C[A-Z2-7]{55}' | tail -1)
DEPLOY_TX=$(echo "$DEPLOY_LOG" | tx_of)
TOKEN=$(stellar contract id asset --asset native --network "$NET")

invoke() { # invoke <source> <fn> [args...]
  local src=$1; shift
  stellar contract invoke --id "$CONTRACT" --source "$src" --network "$NET" -- "$@" 2>&1 | tee /dev/stderr
}

DEADLINE=$(( $(date +%s) + WAIT_SECONDS ))
GOAL_OK=1000000000          # 100 XLM
GOAL_MISS=100000000000      # 10,000 XLM

echo "==> create campaigns"
OUT=$(invoke "$CREATOR" create_campaign --creator "$CREATOR_ADDR" --token "$TOKEN" --goal "$GOAL_OK" --deadline "$DEADLINE")
CREATE0_TX=$(echo "$OUT" | tx_of)
OUT=$(invoke "$CREATOR" create_campaign --creator "$CREATOR_ADDR" --token "$TOKEN" --goal "$GOAL_MISS" --deadline "$DEADLINE")
CREATE1_TX=$(echo "$OUT" | tx_of)

echo "==> contribute"
OUT=$(invoke "$ALICE" contribute --id 0 --contributor "$ALICE_ADDR" --amount 600000000)
ALICE_TX=$(echo "$OUT" | tx_of)
OUT=$(invoke "$BOB" contribute --id 0 --contributor "$BOB_ADDR" --amount 700000000)
BOB_TX=$(echo "$OUT" | tx_of)
RAISED0=$(echo "$OUT" | result_of)
OUT=$(invoke "$BOB" contribute --id 1 --contributor "$BOB_ADDR" --amount 500000000)
BOB1_TX=$(echo "$OUT" | tx_of)

echo "==> waiting for the deadline"
while [ "$(date +%s)" -lt $(( DEADLINE + 25 )) ]; do sleep 5; done

echo "==> withdraw (campaign 0) and refund (campaign 1)"
OUT=$(invoke "$CREATOR" withdraw --id 0)
WITHDRAW_TX=$(echo "$OUT" | tx_of)
WITHDRAWN=$(echo "$OUT" | result_of)
OUT=$(invoke "$BOB" refund --id 1 --contributor "$BOB_ADDR")
REFUND_TX=$(echo "$OUT" | tx_of)
REFUNDED=$(echo "$OUT" | result_of)

link() { if [ -n "${1:-}" ]; then echo "[\`${1:0:12}...\`](https://stellar.expert/explorer/testnet/tx/$1)"; else echo "see script output"; fi; }

cat > "$PROOF" <<DOC
# Testnet proof

Produced by \`scripts/demo-testnet.sh\` on Stellar testnet with native XLM, on $(date -u +%Y-%m-%d).

| Item | Value |
|---|---|
| Network | Testnet |
| Contract ID | [\`$CONTRACT\`](https://stellar.expert/explorer/testnet/contract/$CONTRACT) |
| WASM sha256 | \`$WASM_HASH\` |
| Token | native XLM |

| Step | Result | Transaction |
|---|---|---|
| deploy | contract created | $(link "$DEPLOY_TX") |
| create_campaign 0 | goal 100 XLM | $(link "$CREATE0_TX") |
| create_campaign 1 | goal 10,000 XLM | $(link "$CREATE1_TX") |
| contribute (Alice, campaign 0) | 60 XLM | $(link "$ALICE_TX") |
| contribute (Bob, campaign 0) | 70 XLM, total raised $(xlm "${RAISED0:-0}") XLM | $(link "$BOB_TX") |
| contribute (Bob, campaign 1) | 50 XLM | $(link "$BOB1_TX") |
| withdraw (creator, campaign 0) | $(xlm "${WITHDRAWN:-0}") XLM paid to the creator | $(link "$WITHDRAW_TX") |
| refund (Bob, campaign 1) | $(xlm "${REFUNDED:-0}") XLM returned, goal was missed | $(link "$REFUND_TX") |
DOC

python3 scripts/update-proof.py "$PROOF" README.md
echo "==> Wrote $PROOF and updated README.md"
