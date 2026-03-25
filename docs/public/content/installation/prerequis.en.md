+++
title = "Prerequisites"
description = "What you need to install SSH-Frontière"
date = 2026-03-24
weight = 1
+++

# Prerequisites

## Target server

| Element | Detail |
|---------|--------|
| System | Linux x86_64 |
| SSH access | Working `sshd` |
| Service account | A dedicated user (e.g., `forge-runner`) |
| Rescue admin account | An account with `/bin/bash` (never modified) |
| Console access | IPMI, KVM, or cloud console — in case of SSH lockout |

**Important**: always keep a working console access and an admin account with a regular shell. If the SSH-Frontière login shell is misconfigured, you could lose SSH access to the service account.

## Build machine

To compile SSH-Frontière from source:

| Element | Detail |
|---------|--------|
| Rust | Version 1.70 or higher |
| musl target | `x86_64-unknown-linux-musl` (for a static binary) |
| `make` | Optional, for Makefile shortcuts |

### Install the musl target

```bash
rustup target add x86_64-unknown-linux-musl
```

## Alternative: pre-compiled binary

If you don't want to compile, you can download the static binary from the [project releases](https://github.com/nothus-forge/ssh-frontiere/releases). The binary has no system dependency.

---

**Next**: [Compilation from source](@/installation/compilation.md)
