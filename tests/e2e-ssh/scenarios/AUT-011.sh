#!/bin/bash
# AUT-011 — Auth avec token tagué → actions taguées accessibles
# Token runner-e2e (tags=["test"]) accède à test.greet (tags=["test"]) → succès
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

OUTPUT=$(ssh_auth_cmd "$SSH_CMD" "secret-runner-e2e" "runner-e2e" $'test greet name=world\n.\n') || {
    echo "FAIL: impossible d'extraire le nonce" >&2
    exit 1
}

if echo "$OUTPUT" | grep -q '"status_code":0' && echo "$OUTPUT" | grep -q "hello"; then
    echo "PASS: token tagué accède à action avec tag matching"
else
    echo "FAIL: token tagué rejeté pour action avec tag matching" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
