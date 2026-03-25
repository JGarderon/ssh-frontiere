#!/bin/bash
# LOG-003 — Commentaires loggés (log_comments=true)
set -euo pipefail

BEFORE=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)

$SSH_CMD_NOAUTH <<'EOF' >/dev/null 2>/dev/null
# test-comment-log-003
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

if echo "$TAIL_OUTPUT" | grep -q '"event":"client_comment"' && echo "$TAIL_OUTPUT" | grep -q 'test-comment-log-003'; then
    echo "PASS: commentaire client loggé avec event=client_comment"
else
    echo "FAIL: commentaire non trouvé dans le log" >&2
    echo "LOG (tail): $TAIL_OUTPUT" >&2
    exit 1
fi
