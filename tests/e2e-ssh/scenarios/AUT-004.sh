#!/bin/bash
# AUT-004 — Auth proof invalide
set -euo pipefail

OUTPUT=$($SSH_CMD <<'EOF' 2>/dev/null
+ auth token=runner-e2e proof=invalide

test echo
.
EOF
) || true

if echo "$OUTPUT" | grep -q "authentication failed"; then
    echo "PASS: proof invalide rejeté avec 'authentication failed'"
else
    echo "FAIL: proof invalide non détecté" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
