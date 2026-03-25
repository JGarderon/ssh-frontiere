#!/bin/bash
# PRO-011 — Arguments nommés : test greet name=e2e → succès, output "hello e2e"
# Vérifie que la syntaxe key=value fonctionne correctement (ADR 0009)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

OUTPUT=$(ssh_auth_cmd "$SSH_CMD" "secret-runner-e2e" "runner-e2e" $'test greet name=e2e\n.\n') || {
    echo "FAIL: impossible d'extraire le nonce" >&2
    exit 1
}

if echo "$OUTPUT" | grep -q '"status_code":0' && echo "$OUTPUT" | grep -q "hello e2e"; then
    echo "PASS: argument nommé name=e2e accepté, output hello e2e"
else
    echo "FAIL: argument nommé name=e2e non accepté ou output incorrect" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
