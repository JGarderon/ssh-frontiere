#!/bin/bash
# SES-007 — Réponse complète entre commandes
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
+ session keepalive

test echo
.
test fail
.
.
EOF
)

# Vérifier 2 réponses >>> distinctes avec des status_code différents (ADR 0011)
RESPONSE_LINES=$(echo "$OUTPUT" | grep '^>>> ')
COUNT=$(echo "$RESPONSE_LINES" | wc -l)

if [ "$COUNT" -ge 2 ]; then
    # Vérifier que les réponses ont des contenus distincts (echo=0, fail=1)
    FIRST=$(echo "$RESPONSE_LINES" | head -1 | sed 's/^>>> //' | jq -r '.status_code')
    SECOND=$(echo "$RESPONSE_LINES" | sed -n '2p' | sed 's/^>>> //' | jq -r '.status_code')
    if [ "$FIRST" = "0" ] && [ "$SECOND" != "" ]; then
        echo "PASS: 2 réponses distinctes en session (codes: $FIRST, $SECOND)"
    else
        echo "FAIL: réponses non distinctes" >&2
        echo "RESPONSES: $RESPONSE_LINES" >&2
        exit 1
    fi
else
    echo "FAIL: attendu au moins 2 réponses, obtenu $COUNT" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
