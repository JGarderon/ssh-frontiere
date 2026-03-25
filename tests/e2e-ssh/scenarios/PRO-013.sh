#!/bin/bash
# PRO-013 — help sans préfixe $ → retour texte humain (#>) + réponse JSON finale (>>>)
# TODO-028 : taper "help" seul (sans $) doit retourner du texte lisible.
# ADR 0011 : >> = stdout streaming, >>> = réponse JSON finale. Help ne streame pas (pas de >>).
set -euo pipefail

OUTPUT=$($SSH_CMD <<'EOF' 2>/dev/null
help
.
EOF
) || true

# Le texte doit contenir "ssh-frontiere" et "Protocol" (lignes #>)
if ! echo "$OUTPUT" | grep -q "ssh-frontiere"; then
    echo "FAIL: help without prefix did not return expected text (ssh-frontiere missing)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi

if ! echo "$OUTPUT" | grep -q "Protocol"; then
    echo "FAIL: help without prefix did not return expected text (Protocol missing)" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi

# Pas de lignes >> (stdout streaming) — help ne streame pas
if echo "$OUTPUT" | grep -q '^>> [^>]'; then
    echo "FAIL: help a produit des lignes de streaming stdout (>> )" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi

# Une ligne >>> (réponse JSON finale) DOIT exister avec status_code 0
RESPONSE_LINE=$(echo "$OUTPUT" | grep '^>>> ' | head -1)
if [ -z "$RESPONSE_LINE" ]; then
    echo "FAIL: aucune ligne de réponse JSON finale (>>> ) trouvée" >&2
    echo "OUTPUT: $OUTPUT" >&2
    exit 1
fi

JSON_PART=$(echo "$RESPONSE_LINE" | sed 's/^>>> //')
if echo "$JSON_PART" | jq -e '.status_code == 0' >/dev/null 2>&1; then
    echo "PASS: help sans préfixe retourne du texte humain (#>) et une réponse JSON finale (>>> ) avec status_code=0"
else
    echo "FAIL: réponse JSON finale n'a pas status_code=0" >&2
    echo "JSON: $JSON_PART" >&2
    exit 1
fi
