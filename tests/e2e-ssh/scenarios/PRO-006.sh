#!/bin/bash
# PRO-006 — Réponse JSON 5 champs
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
test echo
.
EOF
)

# Extraire la ligne >>> JSON (ADR 0011 : réponse JSON préfixée >>> )
JSON_LINE=$(echo "$OUTPUT" | grep '^>>> ' | head -1 | sed 's/^>>> //')

if [ -z "$JSON_LINE" ]; then
    echo "FAIL: aucune ligne >>> trouvée" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi

# Vérifier les 5 champs obligatoires
for field in command status_code status_message stdout stderr; do
    if ! echo "$JSON_LINE" | jq -e "has(\"$field\")" >/dev/null 2>&1; then
        echo "FAIL: champ '$field' manquant dans la réponse JSON" >&2
        echo "JSON: $JSON_LINE" >&2
        exit 1
    fi
done

echo "PASS: réponse JSON contient les 5 champs command, status_code, status_message, stdout, stderr"
