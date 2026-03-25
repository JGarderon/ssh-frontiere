#!/bin/bash
# ROB-004 — Connexions simultanées
set -euo pipefail

PIDS=""
TMPDIR=$(mktemp -d)
TOTAL=5

# Lancer 5 connexions en parallèle
for i in $(seq 1 $TOTAL); do
    (
        OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
test echo
.
EOF
        ) || true
        echo "$OUTPUT" > "$TMPDIR/result_$i"
    ) &
    PIDS="$PIDS $!"
done

# Attendre toutes les connexions
FAIL_COUNT=0
for pid in $PIDS; do
    wait "$pid" || true
done

# Vérifier les résultats
SUCCESS=0
for i in $(seq 1 $TOTAL); do
    if [ -f "$TMPDIR/result_$i" ] && grep -q '"status_code":0' "$TMPDIR/result_$i"; then
        SUCCESS=$((SUCCESS + 1))
    fi
done

rm -rf "$TMPDIR"

if [ "$SUCCESS" -eq "$TOTAL" ]; then
    echo "PASS: $TOTAL connexions simultanées, toutes avec réponse valide"
else
    echo "FAIL: seulement $SUCCESS/$TOTAL connexions réussies" >&2
    exit 1
fi
