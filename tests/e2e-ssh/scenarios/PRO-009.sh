#!/bin/bash
# PRO-009 — Préfixe >>> sur la réponse JSON (ADR 0011)
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
test echo
.
EOF
)

# Vérifier qu'une ligne commence par ">>> " suivi de JSON valide (ADR 0011)
RESPONSE_LINE=$(echo "$OUTPUT" | grep '^>>> ' | head -1)

if [ -z "$RESPONSE_LINE" ]; then
    echo "FAIL: aucune ligne avec préfixe '>>> ' trouvée" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi

JSON_PART=$(echo "$RESPONSE_LINE" | sed 's/^>>> //')
if echo "$JSON_PART" | jq . >/dev/null 2>&1; then
    echo "PASS: réponse commence par '>>> ' suivi de JSON valide"
else
    echo "FAIL: contenu après '>>> ' n'est pas du JSON valide" >&2
    echo "LINE: $RESPONSE_LINE" >&2
    exit 1
fi
