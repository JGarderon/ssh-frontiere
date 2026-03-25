#!/bin/bash
# SEC-006 — Opérateur ; (séquentiel strict) : commande non autorisée rejetée
set -euo pipefail

# En v2, ; est le séquentiel strict. test echo réussit, puis /bin/id
# est tenté mais rejeté (domaine non autorisé)
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
test echo ; /bin/id
.
EOF
) || true

if echo "$OUTPUT" | grep -q '"status_code":128'; then
    echo "PASS: commande non autorisée rejetée via opérateur ; (status_code=128)"
else
    echo "FAIL: commande non autorisée non rejetée via opérateur ;" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
