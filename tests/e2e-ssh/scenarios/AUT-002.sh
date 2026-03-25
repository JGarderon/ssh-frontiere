#!/bin/bash
# AUT-002 — Action level=ops sans +auth → rejet
set -euo pipefail

# test greet est level=ops, id_read est --level=read, pas d'auth → rejet
OUTPUT=$($SSH_CMD <<'EOF' 2>/dev/null
test greet name=world
.
EOF
) || true

if echo "$OUTPUT" | grep -q '"status_code":131'; then
    echo "PASS: action level=ops rejetée sans auth (status_code=131)"
else
    echo "FAIL: réponse inattendue pour action ops sans auth" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
