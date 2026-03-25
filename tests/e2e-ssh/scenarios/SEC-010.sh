#!/bin/bash
# SEC-010 — Commande inconnue rejetée
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
unknown action
.
EOF
) || true

if echo "$OUTPUT" | grep -q '"status_code":128'; then
    echo "PASS: commande inconnue rejetée (status_code=128)"
else
    echo "FAIL: commande inconnue non rejetée ou code inattendu" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
