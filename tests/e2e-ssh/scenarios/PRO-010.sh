#!/bin/bash
# PRO-010 — Flush de la bannière
set -euo pipefail

OUTPUT=$(timeout 5 $SSH_CMD < /dev/null 2>/dev/null || true)

if echo "$OUTPUT" | grep -q "#> ssh-frontiere"; then
    echo "PASS: bannière reçue avant fermeture (flush fonctionne)"
else
    echo "FAIL: bannière non reçue avec /dev/null en stdin" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
