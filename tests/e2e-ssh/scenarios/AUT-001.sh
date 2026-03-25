#!/bin/bash
# AUT-001 — Action level=read sans +auth → OK
set -euo pipefail

# test echo est level=read, id_read est --level=read → pas besoin d'auth
OUTPUT=$($SSH_CMD <<'EOF'
test echo
.
EOF
)

if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: action level=read sans auth acceptée"
else
    echo "FAIL: action level=read rejetée sans auth" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
