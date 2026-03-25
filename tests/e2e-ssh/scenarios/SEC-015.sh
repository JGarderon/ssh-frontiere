#!/bin/bash
# SEC-015 — Tags empêchent l'exécution cross-domaine
# Token runner-e2e (tags=["test"]) tente d'accéder à admin.status (tags=["admin"]) → rejeté
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

OUTPUT=$(ssh_auth_cmd "$SSH_CMD" "secret-runner-e2e" "runner-e2e" $'admin status\n.\n') || {
    echo "FAIL: impossible d'extraire le nonce" >&2
    exit 1
}

if echo "$OUTPUT" | grep -q "tag mismatch"; then
    echo "PASS: tags empêchent l'accès cross-domaine (runner-e2e → admin.status)"
else
    echo "FAIL: accès cross-domaine non bloqué par les tags" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
