#!/bin/bash
# ROB-005 — UTF-8 dans commentaires
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
test echo
.
EOF
)

# Vérifier que le protocole fonctionne (les commentaires UTF-8 sont côté client headers)
# Testons avec un commentaire UTF-8 dans les headers
OUTPUT2=$($SSH_CMD_NOAUTH <<'EOF'
# éèêë « » àùô ñ
# 日本語テスト
test echo
.
EOF
)

if echo "$OUTPUT2" | grep -q '"status_code":0'; then
    echo "PASS: commentaires UTF-8 transmis sans corruption"
else
    echo "FAIL: commentaires UTF-8 ont perturbé le protocole" >&2
    echo "OUTPUT: $OUTPUT2" >&2
    exit 1
fi
