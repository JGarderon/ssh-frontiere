#!/bin/bash
# SES-004 — "." seul ferme la session
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
+ session keepalive

test echo
.
.
EOF
)

if echo "$OUTPUT" | grep -q "session closed"; then
    echo "PASS: \".\" seul ferme la session avec '#> session closed'"
else
    echo "FAIL: '#> session closed' absent après \".\"" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
