#!/bin/bash
# ROB-003 — Réponse volumineuse (big-output = seq 1 10000)
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
test big-output
.
EOF
)

# Vérifier que la réponse est du JSON valide malgré la sortie volumineuse
JSON_LINE=$(echo "$OUTPUT" | grep '^>>> ' | head -1 | sed 's/^>>> //')

if [ -z "$JSON_LINE" ]; then
    echo "FAIL: aucune réponse JSON" >&2
    echo "OUTPUT (tronqué): $(echo "$OUTPUT" | head -5)" >&2
    exit 1
fi

if echo "$JSON_LINE" | jq -e '.status_code == 0' >/dev/null 2>&1; then
    STDOUT_LEN=$(echo "$JSON_LINE" | jq -r '.stdout // ""' | wc -c)
    echo "PASS: réponse volumineuse reçue en JSON valide (stdout=${STDOUT_LEN} chars)"
else
    echo "FAIL: réponse JSON invalide ou status_code != 0" >&2
    echo "JSON (tronqué): $(echo "$JSON_LINE" | head -c 200)" >&2
    exit 1
fi
