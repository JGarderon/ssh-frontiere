#!/bin/bash
# SEC-012 — Commande avec domaine inconnu rejetée
set -euo pipefail

# En v2, les lignes sans préfixe sont des commandes. forgejo n'est pas
# un domaine configuré → rejet (status_code=128)
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
forgejo healthcheck
.
EOF
) || true

if echo "$OUTPUT" | grep -q '"status_code":128'; then
    echo "PASS: domaine inconnu rejeté (status_code=128)"
else
    echo "FAIL: domaine inconnu non rejeté" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
