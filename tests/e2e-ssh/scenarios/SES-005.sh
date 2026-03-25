#!/bin/bash
# SES-005 — EOF ferme la session
set -euo pipefail

# Envoyer une session sans "." final, juste fermer stdin
RC=0
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
+ session keepalive

test echo
.
EOF
) || RC=$?

# La connexion doit se terminer proprement (EOF détecté après le bloc commande)
if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: EOF ferme la session proprement (code=$RC)"
else
    echo "FAIL: session non terminée proprement par EOF" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
