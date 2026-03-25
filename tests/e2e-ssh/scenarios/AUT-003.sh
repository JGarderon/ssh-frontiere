#!/bin/bash
# AUT-003 — Auth valide + action level=ops
set -euo pipefail

OUT=$(mktemp)
FIFO=$(mktemp -u)
mkfifo "$FIFO"
trap 'rm -f "$FIFO" "$OUT"' EXIT

# Background: poll for nonce, compute proof, write auth + command
{
    set +e
    NONCE=""
    for i in $(seq 1 50); do
        NONCE=$(sed -n 's/^+> challenge nonce=\([0-9a-f]*\)/\1/p' "$OUT" 2>/dev/null | head -1)
        [ -n "$NONCE" ] && break
        sleep 0.1
    done
    if [ -n "$NONCE" ]; then
        PROOF=$("$PROOF_BIN" --secret "secret-runner-e2e" --nonce "$NONCE")
        printf '+ auth token=runner-e2e proof=%s\n\ntest greet name=world\n.\n' "$PROOF"
    else
        printf '.\n'
    fi
} > "$FIFO" &

$SSH_CMD < "$FIFO" > "$OUT" 2>/dev/null || true
wait 2>/dev/null || true

OUTPUT=$(cat "$OUT")

if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: auth valide, action ops acceptee"
else
    echo "FAIL: auth valide mais action ops rejetee" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
