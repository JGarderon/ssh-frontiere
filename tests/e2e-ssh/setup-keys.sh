#!/bin/bash
# Generates SSH keys for E2E tests in .keys/ directory.
# Called before docker/podman compose up to avoid depends_on timing issues.
set -e

KEYS_DIR="$(dirname "$0")/.keys"

if [ -f "$KEYS_DIR/id_read" ]; then
    echo "[setup-keys] Keys already exist in $KEYS_DIR, skipping generation."
    exit 0
fi

echo "[setup-keys] Generating SSH keys in $KEYS_DIR ..."
mkdir -p "$KEYS_DIR"

ssh-keygen -t ed25519 -f "$KEYS_DIR/id_read" -N "" -q
ssh-keygen -t ed25519 -f "$KEYS_DIR/id_ops" -N "" -q
ssh-keygen -t ed25519 -f "$KEYS_DIR/id_noauth" -N "" -q
ssh-keygen -t ed25519 -f "$KEYS_DIR/id_simpleauth" -N "" -q

chmod 644 "$KEYS_DIR"/*

echo "[setup-keys] Done. Keys generated in $KEYS_DIR"
