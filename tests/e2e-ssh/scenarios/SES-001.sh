#!/bin/bash
# SES-001 — +session keepalive → connexion ouverte
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

# Vérifier qu'au moins 2 réponses >>> sont présentes (ADR 0011)
COUNT=$(echo "$OUTPUT" | grep -c '^>>> ' || true)

if [ "$COUNT" -ge 2 ]; then
    echo "PASS: session keepalive, $COUNT réponses reçues"
else
    echo "FAIL: session keepalive, seulement $COUNT réponse(s)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
