#!/bin/bash
# SEC-016 — Tags RBAC fusionnés correctement en session
# Token admin-e2e (tags=["test","admin"]) peut accéder à test.greet ET admin.status
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"

# Session mode : auth + session keepalive, puis deux commandes
OUTPUT=$(ssh_auth_cmd "$SSH_CMD" "secret-admin-e2e" "admin-e2e" $'+ session keepalive\ntest greet name=world\n.\nadmin status\n.\n.\n') || {
    echo "FAIL: impossible d'extraire le nonce" >&2
    exit 1
}

# Les deux commandes doivent réussir (status_code 0)
RESPONSE_COUNT=$(echo "$OUTPUT" | grep -c '"status_code":0' || true)

if [ "$RESPONSE_COUNT" -ge 2 ]; then
    echo "PASS: tags fusionnés admin-e2e accède à test.greet ET admin.status"
else
    echo "FAIL: token multi-tags n'accède pas aux deux domaines" >&2
    echo "RESPONSE_COUNT: $RESPONSE_COUNT" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
