#!/bin/bash
# PRO-004 — Entêtes client acceptés
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
# commentaire test
test echo
.
EOF
)

if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: entêtes client acceptés, réponse status_code=0"
else
    echo "FAIL: réponse inattendue" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
