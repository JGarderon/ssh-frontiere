#!/bin/bash
# SEC-011 — Arguments excédentaires rejetés
set -euo pipefail

# test echo attend 0 arguments
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
test echo arg1 arg2 arg3
.
EOF
) || true

if echo "$OUTPUT" | grep -q '"status_code":128'; then
    echo "PASS: arguments excédentaires rejetés (status_code=128)"
else
    echo "FAIL: arguments excédentaires non rejetés ou code inattendu" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
