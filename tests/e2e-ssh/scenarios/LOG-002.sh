#!/bin/bash
# LOG-002 — SSH_CLIENT capturé dans le log
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

TAIL_OUTPUT=$(tail -n "$NEW_LINES" "$LOG_FILE")

if echo "$TAIL_OUTPUT" | grep '"event":"executed"' | grep -q '"ssh_client"'; then
    echo "PASS: ssh_client présent dans le log"
else
    echo "FAIL: ssh_client absent du log" >&2
    echo "LOG (tail): $TAIL_OUTPUT" >&2
    exit 1
fi
