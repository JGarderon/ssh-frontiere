#!/bin/bash
# SES-003 — Auth + commande en session
set -euo pipefail

OUT=$(mktemp)
FIFO=$(mktemp -u)
mkfifo "$FIFO"
trap 'rm -f "$FIFO" "$OUT"' EXIT

# Background: poll for nonce, compute proof, send auth + session + command
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
        printf '+ auth token=runner-e2e proof=%s\n+ session keepalive\n\ntest greet name=world\n.\n.\n' "$PROOF"
    else
        printf '.\n'
    fi
} > "$FIFO" &

$SSH_CMD < "$FIFO" > "$OUT" 2>/dev/null || true
wait 2>/dev/null || true

OUTPUT=$(cat "$OUT")

if echo "$OUTPUT" | grep -q "auth ok" && echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: auth + commande ops en session fonctionnent"
else
    echo "FAIL: auth ou commande ops echouee en session" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
