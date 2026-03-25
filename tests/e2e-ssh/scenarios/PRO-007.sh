#!/bin/bash
# PRO-007 — Connexion fermée après réponse (one-shot)
set -euo pipefail

RC=0
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
# test one-shot
test echo
.
EOF
) || RC=$?

# Sans +session, la connexion doit se fermer proprement après la réponse
if [ "$RC" -eq 0 ] && echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: connexion fermée après réponse one-shot (code=0)"
else
    echo "FAIL: code retour=$RC inattendu ou réponse absente" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
