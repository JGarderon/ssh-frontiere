#!/usr/bin/env bash
# BOD-004 — Body dépassant max_body_size → rejet avant exécution
set -euo pipefail

# La config E2E définit max_body_size = 100 pour "test cat-small"
# On envoie un body de 200 octets → doit être rejeté avec une erreur (pas status_code:0)
# Si l'action n'a pas de max_body_size explicite, la limite par défaut est 65536 (64 Ko).
# Ce test utilise "test cat-small" qui a max_body_size = 100 octets.
RC=0
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
+ body
test cat-small
.
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
.
EOF
) || RC=$?

# Le body dépasse max_body_size → le serveur doit rejeter avec un status_code non nul
# ou une réponse d'erreur (pas de status_code:0)
if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "FAIL: body trop grand accepté alors qu'il devrait être rejeté" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
else
    echo "PASS: body dépassant max_body_size correctement rejeté (code=$RC)"
fi
