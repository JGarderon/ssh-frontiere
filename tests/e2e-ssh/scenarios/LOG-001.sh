#!/bin/bash
# LOG-001 — Commande exécutée → entrée JSON dans le log
set -euo pipefail

# Marquer le début (nombre de lignes du log avant la commande)
BEFORE=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)

# Exécuter une commande
$SSH_CMD_NOAUTH <<'EOF' >/dev/null 2>/dev/null
test echo
.
EOF

# Attendre un peu que le log soit écrit
sleep 0.5

# Vérifier les nouvelles entrées
AFTER=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)
NEW_LINES=$((AFTER - BEFORE))

if [ "$NEW_LINES" -le 0 ]; then
    echo "FAIL: aucune nouvelle entrée dans le log" >&2
    exit 1
fi

# Vérifier qu'une entrée "executed" existe dans les nouvelles lignes
TAIL_OUTPUT=$(tail -n "$NEW_LINES" "$LOG_FILE")

if echo "$TAIL_OUTPUT" | grep -q '"event":"executed"'; then
    echo "PASS: commande exécutée loggée avec event=executed"
else
    echo "FAIL: event=executed absent du log" >&2
    echo "LOG (tail): $TAIL_OUTPUT" >&2
    exit 1
fi
