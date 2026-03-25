#!/bin/bash
# PRO-003 — Challenge absent si auth non configurée
set -euo pipefail

OUTPUT=$(echo "" | $SSH_CMD_NOAUTH 2>/dev/null || true)

if echo "$OUTPUT" | grep -q "+> challenge"; then
    echo "FAIL: challenge présent alors qu'auth non configurée" >&2
    exit 1
fi

if echo "$OUTPUT" | grep -q "rbac"; then
    echo "FAIL: rbac présent dans capabilities sans auth" >&2
    exit 1
fi

echo "PASS: pas de challenge ni rbac sans auth configurée"
