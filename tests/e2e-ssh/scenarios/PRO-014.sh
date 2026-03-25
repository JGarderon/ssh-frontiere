#!/bin/bash
# PRO-014 — Commande sans ligne vide entre entêtes et commande → exécution OK
# TODO-027 : la ligne vide est optionnelle. La commande suit directement les entêtes.
# Note : le client n'envoie PAS de préfixe "$" — c'est une notation documentaire (ADR 0006).
set -euo pipefail

# Envoi direct : pas de ligne vide entre la fin des entêtes et la commande
OUTPUT=$($SSH_CMD <<'EOF' 2>/dev/null
test echo
.
EOF
) || true

# La commande doit s'exécuter normalement
if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: commande sans ligne vide exécutée correctement"
else
    echo "FAIL: commande sans ligne vide a échoué" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
