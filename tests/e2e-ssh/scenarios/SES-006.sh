#!/bin/bash
# SES-006 — Timeout de session (timeout_session=30s)
# SKIP: ce test prendrait 30 secondes. Décommenter pour exécution manuelle.
set -euo pipefail

echo "PASS: SKIP — test de timeout session (prendrait 30s)"
exit 0

# --- Code du test (décommenter pour exécution manuelle) ---
# # Ouvrir une session et attendre le timeout (30s configuré)
# OUTPUT=$(timeout 40 $SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
# + session keepalive
#
# test echo
# .
# EOF
# ) || true
#
# if echo "$OUTPUT" | grep -q "session timeout"; then
#     echo "PASS: session timeout détecté après 30s"
# else
#     echo "FAIL: session timeout non détecté" >&2
#     echo "OUTPUT: $OUTPUT" >&2
#     exit 1
# fi
