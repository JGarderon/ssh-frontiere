#!/bin/bash
# SES-008 — Commentaires # en session ignorés
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
+ session keepalive

test echo
.
# note entre commandes
test echo
.
.
EOF
)

# Vérifier que les 2 commandes echo ont produit des réponses >>> (le # est ignoré, ADR 0011)
COUNT=$(echo "$OUTPUT" | grep -c '^>>> ' || true)

if [ "$COUNT" -ge 2 ]; then
    echo "PASS: commentaires # en session ignorés, $COUNT réponses reçues"
else
    echo "FAIL: commentaires en session ont perturbé le flux ($COUNT réponses)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
