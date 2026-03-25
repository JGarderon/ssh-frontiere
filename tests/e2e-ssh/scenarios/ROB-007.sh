#!/bin/bash
# ROB-007 — Ligne vide en session entre commandes
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
+ session keepalive

test echo
.


test echo
.
.
EOF
)

# Les lignes vides en session sont ignorées (read_session_input skippe EmptyLine, ADR 0011)
COUNT=$(echo "$OUTPUT" | grep -c '^>>> ' || true)

if [ "$COUNT" -ge 2 ]; then
    echo "PASS: lignes vides en session ignorées, $COUNT réponses reçues"
else
    echo "FAIL: lignes vides ont perturbé la session ($COUNT réponses)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
