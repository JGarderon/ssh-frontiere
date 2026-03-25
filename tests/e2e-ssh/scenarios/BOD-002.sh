#!/usr/bin/env bash
# BOD-002 — Body mode size=N : lecture d'exactement N octets depuis stdin
set -euo pipefail

# Envoyer +body size=5 dans les entêtes : seuls les 5 premiers octets seront lus
# "hello" fait exactement 5 octets (sans newline final dans le body)
# La commande "test cat" exécute /bin/cat → stdout = les 5 octets du body
RC=0
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
+ body size=5
test cat
.
hello
EOF
) || RC=$?

# Le body (exactement 5 octets "hello") doit être retourné dans stdout
if echo "$OUTPUT" | grep -q '"status_code":0' && echo "$OUTPUT" | grep -q 'hello'; then
    echo "PASS: body mode size=5 transmis via stdin, reçu dans stdout (code=$RC)"
else
    echo "FAIL: body mode size=N non transmis ou status_code non nul (code=$RC)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
