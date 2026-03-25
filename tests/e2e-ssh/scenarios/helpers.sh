#!/bin/bash
# helpers.sh — Fonctions utilitaires pour les tests E2E avec auth nonce
# Source depuis les scripts de test : source "$SCRIPT_DIR/helpers.sh"
# NON exécutable (mode 644) pour ne pas être lancé comme test par run-all.sh

# ssh_auth_cmd <ssh_cmd> <secret> <token_id> <body>
# Ouvre une session SSH unique, extrait le nonce de la bannière,
# calcule le proof, envoie +auth puis le body.
# Affiche le résultat complet sur stdout.
# Retourne 0 si OK, 1 si le nonce n'a pas été trouvé.
ssh_auth_cmd() {
    local ssh_cmd="$1"
    local secret="$2"
    local token_id="$3"
    local body="$4"

    local tmpdir
    tmpdir=$(mktemp -d)
    local in_fifo="$tmpdir/in"
    local out_file="$tmpdir/out"
    mkfifo "$in_fifo"

    # Démarrer SSH : stdin depuis le FIFO, stdout vers fichier
    eval "$ssh_cmd" < "$in_fifo" > "$out_file" 2>/dev/null &
    local ssh_pid=$!

    # Ouvrir le FIFO en écriture (débloque le read côté SSH)
    exec 7>"$in_fifo"

    # Attendre le nonce dans la bannière serveur (max 5s)
    local nonce=""
    local i
    for i in $(seq 1 50); do
        nonce=$(sed -n 's/^+> challenge nonce=\([0-9a-f]*\)/\1/p' "$out_file" 2>/dev/null | head -1) || true
        [ -n "$nonce" ] && break || true
        sleep 0.1
    done

    if [ -z "$nonce" ]; then
        exec 7>&-
        wait "$ssh_pid" 2>/dev/null || true
        rm -rf "$tmpdir"
        echo "ERROR: nonce not found in banner" >&2
        return 1
    fi

    # Calculer le proof et envoyer auth + body
    local proof
    proof=$("$PROOF_BIN" --secret "$secret" --nonce "$nonce")
    printf '+ auth token=%s proof=%s\n' "$token_id" "$proof" >&7
    printf '%s' "$body" >&7
    exec 7>&-

    # Attendre la fin de la session SSH
    wait "$ssh_pid" 2>/dev/null || true
    cat "$out_file"
    rm -rf "$tmpdir"
}
