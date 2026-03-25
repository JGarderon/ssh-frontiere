#!/bin/bash
# AUT-009 — Token inconnu rejeté
set -euo pipefail

OUTPUT=$($SSH_CMD <<'EOF' 2>/dev/null
+ auth token=inexistant proof=abc123

test echo
.
EOF
) || true

if echo "$OUTPUT" | grep -q "auth failed\|unknown token"; then
    echo "PASS: token inconnu rejeté"
else
    echo "FAIL: token inconnu non rejeté" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
