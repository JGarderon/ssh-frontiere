#!/bin/bash
# LOG-005 — Timestamp ISO 8601 dans le log
set -euo pipefail

BEFORE=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)

$SSH_CMD_NOAUTH <<'EOF' >/dev/null 2>/dev/null
test echo
.
EOF

sleep 0.5

AFTER=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)
NEW_LINES=$((AFTER - BEFORE))

if [ "$NEW_LINES" -le 0 ]; then
    echo "FAIL: aucune nouvelle entrée dans le log" >&2
    exit 1
fi

# Vérifier le format timestamp ISO 8601 (YYYY-MM-DDTHH:MM:SS ou YYYY-MM-DD...)
LAST_LINE=$(tail -n 1 "$LOG_FILE")

if echo "$LAST_LINE" | grep -qE '"timestamp":"[0-9]{4}-[0-9]{2}-[0-9]{2}'; then
    echo "PASS: timestamp au format ISO 8601 (YYYY-MM-DD)"
else
    echo "FAIL: timestamp absent ou format invalide" >&2
    echo "LAST: $LAST_LINE" >&2
    exit 1
fi
