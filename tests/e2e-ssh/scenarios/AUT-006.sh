#!/bin/bash
# AUT-006 — Nonce différent à chaque connexion
set -euo pipefail

BANNER1=$(echo "" | $SSH_CMD 2>/dev/null || true)
NONCE1=$(echo "$BANNER1" | sed -n 's/^+> challenge nonce=\([0-9a-f]*\)/\1/p' | head -1)

BANNER2=$(echo "" | $SSH_CMD 2>/dev/null || true)
NONCE2=$(echo "$BANNER2" | sed -n 's/^+> challenge nonce=\([0-9a-f]*\)/\1/p' | head -1)

if [ -z "$NONCE1" ] || [ -z "$NONCE2" ]; then
    echo "FAIL: impossible d'extraire les nonces" >&2
    exit 1
fi

if [ "$NONCE1" != "$NONCE2" ]; then
    echo "PASS: nonces differents entre deux connexions"
else
    echo "FAIL: nonces identiques ($NONCE1)" >&2
    exit 1
fi
