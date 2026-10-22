#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --quiet
demo_root=$(mktemp -d "${TMPDIR:-/tmp}/merklelsm-demo.XXXXXX")
trap 'rm -rf -- "$demo_root"' EXIT
ledger=(target/debug/merklelsm --db "$demo_root/db")
"${ledger[@]}" init
"${ledger[@]}" record --id tx-001 --from Alice --to Bob --amount-paise 1050 --timestamp-ms 1700000000000 --note Lunch
"${ledger[@]}" transaction tx-001
if "${ledger[@]}" record --id tx-001 --from Alice --to Bob --amount-paise 1050 --timestamp-ms 1700000000000; then
    echo "ERROR: duplicate transaction was accepted" >&2
    exit 1
fi
"${ledger[@]}" put account-note reviewed
"${ledger[@]}" flush
"${ledger[@]}" put temporary remove-me
"${ledger[@]}" delete temporary
"${ledger[@]}" compact
"${ledger[@]}" scan
"${ledger[@]}" stats
"${ledger[@]}" checkpoint "$demo_root/checkpoint.json"
"${ledger[@]}" verify --checkpoint "$demo_root/checkpoint.json"
"${ledger[@]}" tamper --table 0 --block 0 --allow-demo-tamper
if "${ledger[@]}" verify >"$demo_root/verify.out" 2>&1; then
    echo "ERROR: corruption was accepted" >&2
    exit 1
fi
cat "$demo_root/verify.out"
if ! grep -q 'data block 0' "$demo_root/verify.out"; then
    echo "ERROR: expected a data block 0 diagnostic" >&2
    exit 1
fi
echo "PASS: persistence, duplicate rejection, compaction, checkpoint, and tamper detection"
