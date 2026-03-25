#!/bin/bash
# AUT-008 — Proof d'une connexion précédente rejeté (replay protection)
set -euo pipefail

# Connexion 1 : capturer le nonce et calculer le proof
BANNER1=$(echo "" | $SSH_CMD 2>/dev/null || true)
NONCE1=$(echo "$BANNER1" | sed -n 's/^+> challenge nonce=\([0-9a-f]*\)/\1/p' | head -1)

if [ -z "$NONCE1" ]; then
    echo "FAIL: impossible d'extraire le nonce de la connexion 1" >&2
    exit 1
fi

PROOF1=$($PROOF_BIN --secret "secret-runner-e2e" --nonce "$NONCE1")

# Connexion 2 : envoyer le proof de la connexion 1 (nonce different → rejet)
OUTPUT=$($SSH_CMD <<EOF 2>/dev/null
+ auth token=runner-e2e proof=$PROOF1

test greet name=world
.
EOF
) || true

if echo "$OUTPUT" | grep -q "authentication failed"; then
    echo "PASS: proof d'une connexion precedente rejete (replay detecte)"
else
    # Vérifier que le nonce a bien changé
    BANNER2_NONCE=$(echo "$OUTPUT" | sed -n 's/^+> challenge nonce=\([0-9a-f]*\)/\1/p' | head -1)
    if [ "$NONCE1" = "$BANNER2_NONCE" ]; then
        echo "FAIL: les nonces sont identiques (pas de replay protection)" >&2
    else
        echo "FAIL: proof ancien accepte malgre nonce different" >&2
    fi
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
