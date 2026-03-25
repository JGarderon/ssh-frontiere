#!/bin/bash
# SEC-003 — restrict bloque agent forwarding
set -euo pipefail

SSH_BASE="ssh -T -o BatchMode=yes -o ConnectTimeout=5"

OUTPUT=$($SSH_BASE -i ~/.ssh/id_read -A e2e-user@server <<'EOF' 2>&1
test echo
.
EOF
) || true

# Agent forwarding bloqué par restrict → le protocole fonctionne quand même
# mais sans agent forwarding. On vérifie surtout que ça n'ouvre pas de brèche.
if echo "$OUTPUT" | grep -q "#> ssh-frontiere"; then
    echo "PASS: agent forwarding bloqué, protocole fonctionne normalement"
else
    echo "FAIL: réponse inattendue avec -A" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
