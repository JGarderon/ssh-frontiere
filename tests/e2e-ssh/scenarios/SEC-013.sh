#!/bin/bash
# SEC-013 — command= force le binaire ssh-frontiere
set -euo pipefail

SSH_BASE="ssh -T -o BatchMode=yes -o ConnectTimeout=5"

# Tenter d'exécuter /bin/bash via SSH → command= dans authorized_keys force ssh-frontiere
OUTPUT=$($SSH_BASE -i ~/.ssh/id_read e2e-user@server /bin/bash <<'EOF' 2>&1
EOF
) || true

# ssh-frontiere doit s'exécuter (bannière visible), pas /bin/bash
if echo "$OUTPUT" | grep -q "#> ssh-frontiere"; then
    echo "PASS: command= force ssh-frontiere, /bin/bash ignoré"
else
    echo "FAIL: bannière ssh-frontiere absente (command= contourné ?)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
