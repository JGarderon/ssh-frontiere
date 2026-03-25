#!/usr/bin/env bash
# BOD-003 — Body mode stop="FIN" : lecture jusqu'au séparateur personnalisé
set -euo pipefail

# Envoyer +body stop="FIN" dans les entêtes
# La lecture du body s'arrête quand la ligne "FIN" est rencontrée seule
# La commande "test cat" exécute /bin/cat → stdout = contenu du body avant "FIN"
RC=0
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
+ body stop="FIN"
test cat
.
ligne un
ligne deux
FIN
EOF
) || RC=$?

# Le body ("ligne un\nligne deux\n") doit être retourné dans stdout
if echo "$OUTPUT" | grep -q '"status_code":0' && echo "$OUTPUT" | grep -q 'ligne un'; then
    echo "PASS: body mode stop=\"FIN\" transmis via stdin, reçu dans stdout (code=$RC)"
else
    echo "FAIL: body mode stop non transmis ou status_code non nul (code=$RC)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
