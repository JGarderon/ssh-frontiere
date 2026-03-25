#!/bin/bash
# ROB-002 — Commande très longue (> 4096 chars)
set -euo pipefail

# Générer une commande avec un argument de 4080+ caractères
LONG_ARG=$(printf 'A%.0s' $(seq 1 4080))

EXIT_CODE=0
OUTPUT=$($SSH_CMD_NOAUTH <<EOF 2>/dev/null
test say "message=$LONG_ARG"
.
EOF
) || EXIT_CODE=$?

# Ligne > 4096 chars → rejet protocolaire (exit 132) ou erreur opaque "service unavailable"
if [ "$EXIT_CODE" -eq 132 ] || [ "$EXIT_CODE" -eq 128 ] \
    || echo "$OUTPUT" | grep -q '"status_code":128\|"status_code":132' \
    || echo "$OUTPUT" | grep -q "protocol error\|too long\|service unavailable"; then
    echo "PASS: commande très longue rejetée"
else
    echo "FAIL: commande très longue non rejetée" >&2
    echo "EXIT_CODE: $EXIT_CODE" >&2
    echo "OUTPUT (tronqué): $(echo "$OUTPUT" | head -5)" >&2
    exit 1
fi
