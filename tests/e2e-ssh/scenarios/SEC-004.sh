#!/bin/bash
# SEC-004 — restrict bloque PTY, protocole fonctionne quand même
set -euo pipefail

# Sans -T (donc ssh demanderait un PTY par défaut), mais restrict bloque pty-req
SSH_BASE="ssh -o BatchMode=yes -o ConnectTimeout=5"

OUTPUT=$($SSH_BASE -i ~/.ssh/id_noauth e2e-user@server <<'EOF' 2>&1
test echo
.
EOF
) || true

# Le protocole doit fonctionner même si le PTY est refusé
if echo "$OUTPUT" | grep -q "#> ssh-frontiere"; then
    echo "PASS: protocole fonctionne malgré restriction PTY"
else
    echo "FAIL: protocole ne fonctionne pas sans -T" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
