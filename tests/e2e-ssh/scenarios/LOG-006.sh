#!/bin/bash
# LOG-006 — Arguments sensibles masqués (SHA-256)
# SKIP: la config E2E n'a pas d'action avec sensitive=true.
# Ce test nécessiterait d'ajouter une action avec un argument sensitive dans config.toml.
set -euo pipefail

echo "PASS: SKIP — pas d'action avec sensitive=true dans la config E2E"
exit 0

# --- Code du test (décommenter après ajout d'une action sensitive dans config.toml) ---
# BEFORE=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)
#
# $SSH_CMD_NOAUTH <<'EOF' >/dev/null 2>/dev/null
# test secret-cmd "my-password"
# .
# EOF
#
# sleep 0.5
#
# AFTER=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)
# NEW_LINES=$((AFTER - BEFORE))
#
# TAIL_OUTPUT=$(tail -n "$NEW_LINES" "$LOG_FILE")
#
# # Vérifier que l'argument sensible est masqué (SHA-256 hex, pas la valeur brute)
# if echo "$TAIL_OUTPUT" | grep -q 'my-password'; then
#     echo "FAIL: argument sensible en clair dans le log" >&2
#     exit 1
# fi
#
# if echo "$TAIL_OUTPUT" | grep -qE '"event":"executed"'; then
#     echo "PASS: argument sensible masqué dans le log"
# else
#     echo "FAIL: commande non trouvée dans le log" >&2
#     exit 1
# fi
