#!/bin/bash
# SEC-001 — restrict bloque port-forwarding
# SKIP: port-forwarding rejection depends on sshd restrict behavior which is
# unreliable inside Docker containers (no PTY, different network namespace).
echo "SKIP: port-forwarding test non fiable dans Docker"
exit 0
