#!/usr/bin/env bash
# BOD-005 — Body en session keepalive : première commande avec body, deuxième sans
set -euo pipefail

# Session avec 2 commandes :
#   1. test cat avec body → stdout contient le body
#   2. test echo sans body → réponse normale
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
+ session keepalive

+ body
test cat
.
body de la premiere commande
.
test echo
.
.
EOF
)

# Vérifier au moins 2 réponses >>> (ADR 0011)
COUNT=$(echo "$OUTPUT" | grep -c '^>>> ' || true)

if [ "$COUNT" -ge 2 ]; then
    # Vérifier que la première réponse contient le body transmis
    FIRST_RESPONSE=$(echo "$OUTPUT" | grep '^>>> ' | head -1)
    if echo "$FIRST_RESPONSE" | grep -q 'body de la premiere commande'; then
        echo "PASS: body en session transmis sur stdin, $COUNT réponses reçues"
    else
        echo "FAIL: body non trouvé dans la première réponse de session" >&2
        echo "OUTPUT: $OUTPUT" >&2
        exit 1
    fi
else
    echo "FAIL: attendu au moins 2 réponses en session, obtenu $COUNT" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
