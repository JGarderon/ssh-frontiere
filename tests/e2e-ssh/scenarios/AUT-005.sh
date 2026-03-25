#!/bin/bash
# AUT-005 — 3 auth invalides → déconnexion
set -euo pipefail

# En mode session, on peut envoyer plusieurs tentatives d'auth
OUTPUT=$($SSH_CMD <<'EOF' 2>/dev/null
+ auth token=runner-e2e proof=bad1
+ session keepalive

test echo
.
+ auth token=runner-e2e proof=bad2
+ auth token=runner-e2e proof=bad3
EOF
) || true

if echo "$OUTPUT" | grep -q "auth failed (3/3)" && echo "$OUTPUT" | grep -q "session terminated"; then
    echo "PASS: 3 auth invalides → session terminée"
else
    echo "FAIL: lockout après 3 échecs non détecté" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
