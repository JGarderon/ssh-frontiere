#!/bin/bash
# PRO-012 — Argument requis omis : test greet sans name= → rejeté (pas de default)
# L'action test.greet a un arg enum "name" sans valeur par défaut.
# Omettre l'argument doit produire une erreur "missing required argument".
set -euo pipefail

# Connexion avec auth runner-e2e (ops level, tags=["test"])
BANNER=$($SSH_CMD <<'INIT' 2>/dev/null || true
.
INIT
)
NONCE=$(echo "$BANNER" | sed -n 's/^+> challenge nonce=\([0-9a-f]*\)/\1/p' | head -1)

if [ -z "$NONCE" ]; then
    echo "FAIL: impossible d'extraire le nonce" >&2
    exit 1
fi

PROOF=$($PROOF_BIN --secret "secret-runner-e2e" --nonce "$NONCE")

OUTPUT=$($SSH_CMD <<EOF 2>/dev/null
+ auth token=runner-e2e proof=$PROOF
test greet
.
EOF
) || true

if echo "$OUTPUT" | grep -q "missing required argument"; then
    echo "PASS: argument requis omis détecté (missing required argument)"
else
    echo "FAIL: argument requis omis non détecté" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
