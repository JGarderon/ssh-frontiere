#!/bin/bash
# PRO-002 — Challenge présent si auth configurée
set -euo pipefail

OUTPUT=$(echo "" | $SSH_CMD 2>/dev/null || true)

NONCE=$(echo "$OUTPUT" | sed -n 's/^+> challenge nonce=\([0-9a-f]*\)/\1/p' | head -1)

if [ -n "$NONCE" ] && [ ${#NONCE} -ge 32 ]; then
    echo "PASS: challenge nonce present avec ${#NONCE} hex chars"
else
    echo "FAIL: challenge nonce absent ou format invalide" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
