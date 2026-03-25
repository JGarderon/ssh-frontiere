#!/bin/bash
# SES-002 — Plusieurs commandes successives
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
+ session keepalive

test echo
.
test echo
.
test echo
.
.
EOF
)

# Compter les réponses >>> (3 commandes echo, ADR 0011)
COUNT=$(echo "$OUTPUT" | grep -c '^>>> ' || true)

if [ "$COUNT" -ge 3 ]; then
    echo "PASS: 3+ commandes successives en session, $COUNT réponses reçues"
else
    echo "FAIL: attendu au moins 3 réponses, obtenu $COUNT" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
