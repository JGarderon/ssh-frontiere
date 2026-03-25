#!/bin/bash
# PRO-001 — Bannière reçue à la connexion
set -euo pipefail

OUTPUT=$(echo "" | $SSH_CMD 2>/dev/null || true)

if echo "$OUTPUT" | grep -q "#> ssh-frontiere" && echo "$OUTPUT" | grep -q "+> capabilities"; then
    echo "PASS: bannière reçue avec #> ssh-frontiere et +> capabilities"
else
    echo "FAIL: bannière manquante ou incomplète" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
