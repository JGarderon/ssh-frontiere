#!/bin/bash
# AUT-010 — Niveau effectif = max(base, token) : clé id_ops sans +auth
set -euo pipefail

# id_ops est configuré avec --level=ops dans authorized_keys
# test ops-echo est level=ops sans tags → accessible avec la clé ops sans auth
# || true : capturer l'output même si SSH retourne non-zero (diagnostic)
OUTPUT=$($SSH_CMD_OPS <<'EOF'
test ops-echo
.
EOF
) || true

if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: clé id_ops (--level=ops) accède à action ops sans auth"
else
    echo "FAIL: clé id_ops rejetée pour action ops" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
