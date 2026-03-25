#!/bin/bash
# SEC-008 — Cas positif : caractères spéciaux entre guillemets
set -euo pipefail

OUTPUT=$($SSH_CMD_NOAUTH <<'EOF'
test say "message=hello|world;test&foo$bar"
.
EOF
)

if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: caractères spéciaux entre guillemets acceptés"
else
    echo "FAIL: caractères spéciaux entre guillemets rejetés" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
