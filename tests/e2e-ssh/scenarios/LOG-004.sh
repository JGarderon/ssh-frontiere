#!/bin/bash
# LOG-004 — Échec auth loggé
set -euo pipefail

BEFORE=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)

# Envoyer 3 auth invalides pour déclencher le lockout (qui est loggé)
$SSH_CMD <<'EOF' >/dev/null 2>/dev/null || true
+ auth token=runner-e2e proof=bad1
+ session keepalive

test echo
.
+ auth token=runner-e2e proof=bad2
+ auth token=runner-e2e proof=bad3
EOF

sleep 0.5

AFTER=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)
NEW_LINES=$((AFTER - BEFORE))

if [ "$NEW_LINES" -le 0 ]; then
    echo "FAIL: aucune nouvelle entrée dans le log" >&2
    exit 1
fi

TAIL_OUTPUT=$(tail -n "$NEW_LINES" "$LOG_FILE")

if echo "$TAIL_OUTPUT" | grep -q '"event":"auth_lockout"\|auth failed'; then
    echo "PASS: échec auth loggé"
else
    echo "FAIL: échec auth non trouvé dans le log" >&2
    echo "LOG (tail): $TAIL_OUTPUT" >&2
    exit 1
fi
