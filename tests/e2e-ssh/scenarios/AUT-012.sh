#!/bin/bash
# AUT-012 — Mode simple (sans nonce) : auth reussie, commande executee
# Config config-simple-auth.toml : challenge_nonce absent (defaut false)
# Proof = SHA-256(secret) — pas de nonce a lire dans la banniere
set -euo pipefail

# Compute simple proof: SHA-256(secret)
SECRET="secret-runner-e2e"
PROOF=$(echo -n "$SECRET" | sha256sum | cut -d' ' -f1)

OUTPUT=$($SSH_CMD_SIMPLEAUTH <<EOF 2>/dev/null
+ auth token=runner-e2e proof=$PROOF

test echo
.
EOF
) || true

if echo "$OUTPUT" | grep -q '"status_code":0'; then
    echo "PASS: simple auth mode, proof SHA-256(secret) acceptee"
else
    echo "FAIL: simple auth mode, proof rejetee" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi
