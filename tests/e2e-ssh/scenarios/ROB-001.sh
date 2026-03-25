#!/bin/bash
# ROB-001 — Déconnexion brutale client
set -euo pipefail

# Lancer une connexion SSH en background (session keepalive)
$SSH_CMD_NOAUTH <<'EOF' &
+ session keepalive

test echo
.
EOF
SSH_PID=$!

# Attendre un peu que la connexion s'établisse
sleep 1

# Kill brutal du processus SSH client
kill -9 "$SSH_PID" 2>/dev/null || true
wait "$SSH_PID" 2>/dev/null || true

# Attendre que le serveur nettoie
sleep 1

# Vérifier qu'on peut encore se connecter (pas de zombie/blocage)
RC=0
OUTPUT=$(timeout 5 $SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
test echo
.
EOF
) || RC=$?

if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: déconnexion brutale gérée, serveur toujours fonctionnel"
else
    echo "FAIL: serveur non fonctionnel après déconnexion brutale (RC=$RC)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
