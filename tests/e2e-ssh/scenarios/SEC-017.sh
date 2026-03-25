#!/bin/bash
# SEC-017 — Argument positionnel rejeté : test greet world (sans =) → rejeté
# La syntaxe positionnelle n'est pas supportée (ADR 0009), seul key=value est accepté.
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
test greet world
.
EOF
) || true

if echo "$OUTPUT" | grep -q "positional arguments not supported"; then
    echo "PASS: argument positionnel rejeté (positional arguments not supported)"
else
    echo "FAIL: argument positionnel non rejeté" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
