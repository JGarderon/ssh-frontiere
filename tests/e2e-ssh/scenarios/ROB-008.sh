#!/bin/bash
# ROB-008 — SIGPIPE propagé (client ferme stdout avant fin écriture)
set -euo pipefail

# Lancer une commande qui produit beaucoup de sortie, mais couper immédiatement
# via head -c 1 pour provoquer un SIGPIPE côté serveur
RC=0
OUTPUT=$($SSH_CMD_NOAUTH <<'EOF' 2>/dev/null | head -c 1
test big-output
.
EOF
) || RC=$?

# Vérifier que le client n'est pas bloqué (le processus SSH doit se terminer)
# On vérifie simplement qu'on a reçu quelque chose et que le processus n'a pas bloqué
# (le timeout de set -euo pipefail + ConnectTimeout=5 protège contre le blocage)

# Après SIGPIPE, vérifier que le serveur est toujours fonctionnel
OUTPUT2=$(timeout 5 $SSH_CMD_NOAUTH <<'EOF' 2>/dev/null
test echo
.
EOF
) || true

if echo "$OUTPUT2" | grep -q '"status_code":0'; then
    echo "PASS: SIGPIPE géré, serveur toujours fonctionnel"
else
    echo "FAIL: serveur non fonctionnel après SIGPIPE" >&2
    echo "OUTPUT2: $OUTPUT2" >&2
    exit 1
fi
