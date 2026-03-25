#!/bin/bash
# AUT-013 — Mode simple (sans nonce) : mauvaise preuve rejetee
# Config config-simple-auth.toml : challenge_nonce absent (defaut false)
set -euo pipefail

OUTPUT=$($SSH_CMD_SIMPLEAUTH <<'EOF' 2>/dev/null
+ auth token=runner-e2e proof=0000000000000000000000000000000000000000000000000000000000000000

test echo
.
EOF
) || true

if echo "$OUTPUT" | grep -q "authentication failed"; then
    echo "PASS: simple auth mode, mauvaise preuve rejetee"
else
    echo "FAIL: simple auth mode, mauvaise preuve non detectee" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
