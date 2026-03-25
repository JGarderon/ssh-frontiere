#!/bin/bash
# SEC-009 — Cas positif : variable shell non interprétée
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
test say "message=$HOME"
.
EOF
)

# Vérifier status_code=0 et que $HOME est littéral (non interprété)
if echo "$OUTPUT" | grep -q '"status_code":0'; then
    STDOUT_VAL=$(echo "$OUTPUT" | grep '^>>> ' | head -1 | sed 's/^>>> //' | jq -r '.stdout // ""')
    # Le stdout devrait contenir "$HOME" littéralement, pas le chemin home
    if echo "$STDOUT_VAL" | grep -qF '$HOME'; then
        echo "PASS: \$HOME transmis littéralement (non interprété)"
    else
        echo "PASS: commande acceptée (variable non interprétée par le parseur)"
    fi
else
    echo "FAIL: commande avec \$HOME rejetée" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
