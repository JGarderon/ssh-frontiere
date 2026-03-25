#!/bin/bash
# PRO-008 — Commentaires serveur dans la bannière
set -euo pipefail

OUTPUT=$(echo "" | $SSH_CMD 2>/dev/null || true)

if echo "$OUTPUT" | grep -q '#> type "help"'; then
    echo "PASS: commentaire d'aide présent dans la bannière"
else
    echo "FAIL: commentaire '#> type \"help\"' absent de la bannière" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
