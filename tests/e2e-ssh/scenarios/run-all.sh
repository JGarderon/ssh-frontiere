#!/bin/bash
# E2E SSH test runner — executes all scenario scripts and reports results
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PASS=0
FAIL=0
TOTAL=0
FAILURES=""

# SSH helper: default key is id_read, connects to server as e2e-user
SSH_BASE="ssh -T -o BatchMode=yes -o ConnectTimeout=5"
export SSH_CMD="$SSH_BASE -i ~/.ssh/id_read e2e-user@server"
export SSH_CMD_OPS="$SSH_BASE -i ~/.ssh/id_ops e2e-user@server"
export SSH_CMD_NOAUTH="$SSH_BASE -i ~/.ssh/id_noauth e2e-user@server"
export SSH_CMD_SIMPLEAUTH="$SSH_BASE -i ~/.ssh/id_simpleauth e2e-user@server"
export PROOF_BIN="/usr/local/bin/ssh-frontiere-proof"
export LOG_FILE="/var/log/ssh-frontiere/commands.json"

for script in "$SCRIPT_DIR"/*.sh; do
    [ "$(basename "$script")" = "run-all.sh" ] && continue
    [ ! -x "$script" ] && continue

    TOTAL=$((TOTAL + 1))
    name="$(basename "$script" .sh)"

    if output=$("$script" 2>&1); then
        PASS=$((PASS + 1))
        echo "E2E-SSH-${name}  PASS"
    else
        FAIL=$((FAIL + 1))
        FAILURES="${FAILURES}\n  E2E-SSH-${name}: ${output}"
        echo "E2E-SSH-${name}  FAIL  ${output}" >&2
    fi
done

echo ""
echo "TOTAL: ${PASS}/${TOTAL} passed, ${FAIL} failed"

if [ $FAIL -gt 0 ]; then
    echo -e "\nFailures:${FAILURES}" >&2
    exit 1
fi
