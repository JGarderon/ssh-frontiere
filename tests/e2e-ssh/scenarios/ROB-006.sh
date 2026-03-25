#!/bin/bash
# ROB-006 — Entête très long (< 4096 chars)
set -euo pipefail

# Générer un commentaire de 4090 caractères (sous la limite MAX_LINE_LEN de 4096)
LONG_COMMENT=$(printf 'X%.0s' $(seq 1 4088))

OUTPUT=$($SSH_CMD_NOAUTH <<EOF
# $LONG_COMMENT
test echo
.
EOF
)

if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: entête de 4090 chars accepté (sous limite 4096)"
else
    echo "FAIL: entête long rejeté" >&2
    echo "OUTPUT (tronqué): $(echo "$OUTPUT" | head -5)" >&2
    exit 1
fi
