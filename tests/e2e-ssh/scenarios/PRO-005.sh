#!/bin/bash
# PRO-005 — Entête dans un bloc commande → erreur protocole
set -euo pipefail

# En v2, une ligne +/# à l'intérieur d'un bloc commande (entre la première
# ligne texte et le ".") provoque une erreur protocole
RC=0
$SSH_CMD_NOAUTH <<'EOF' >/dev/null 2>&1 || RC=$?
test echo
+ session keepalive
.
EOF

if [ "$RC" -ne 0 ]; then
    echo "PASS: entête dans un bloc commande rejeté (code=$RC)"
else
    echo "FAIL: entête dans un bloc commande accepté (code=0)" >&2
    exit 1
fi
