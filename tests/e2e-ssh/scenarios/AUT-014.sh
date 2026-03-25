#!/bin/bash
# AUT-014 — Mode nonce (challenge_nonce=true) preserve
# Config config.toml : challenge_nonce = true
# La banniere doit contenir +> challenge nonce=
set -euo pipefail

# Connexion avec config.toml (challenge_nonce=true) — lire la banniere
BANNER=$($SSH_CMD <<'INIT' 2>/dev/null || true
.
INIT
)

if echo "$BANNER" | grep -q "+> challenge nonce="; then
    echo "PASS: mode nonce, challenge present dans la banniere"
else
    echo "FAIL: mode nonce, pas de challenge dans la banniere" >&2
    echo "BANNER: $BANNER" >&2
    exit 1
fi
