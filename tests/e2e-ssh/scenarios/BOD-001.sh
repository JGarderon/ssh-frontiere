#!/usr/bin/env bash
# BOD-001 — Body mode défaut (one-shot) : +body suivi d'une commande et d'un body terminé par "."
set -euo pipefail

# Envoyer +body dans les entêtes, puis la commande "test cat", puis le body
# Le body est terminé par "." seul sur une ligne (mode défaut)
# La commande "test cat" exécute /bin/cat → stdout = contenu du body
RC=0
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
+ body
test cat
.
hello from body
world
.
EOF
) || RC=$?

# Le body doit être retourné dans stdout de la réponse
if echo "$OUTPUT" | grep -q '"status_code":0' && echo "$OUTPUT" | grep -q 'hello from body'; then
    echo "PASS: body mode défaut transmis via stdin, reçu dans stdout (code=$RC)"
else
    echo "FAIL: body mode défaut non transmis ou status_code non nul (code=$RC)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
