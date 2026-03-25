#!/bin/bash
# Script de test qui intercepte SIGTERM et ecrit dans un fichier
# avant de se terminer proprement.
# Usage: trap-sigterm.sh <marker-file>
# Le fichier marker est cree a la reception de SIGTERM.
MARKER="${1:-/tmp/ssh-frontiere-sigterm-marker}"
trap 'echo "sigterm_received" > "${MARKER}"; exit 0' TERM
# Boucle infinie — ne se termine que par signal
while true; do
    sleep 0.1
done
