+++
title = "Security"
description = "Security model, guarantees, and limitations of SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Security

SSH-Frontière is a **security component**. Its purpose is to restrict what incoming SSH connections can do. This page documents the security model, what has been implemented, and what is not guaranteed.

## Security model

### Core principle: deny by default

Nothing runs without being explicitly configured. If a command is not in the TOML whitelist, it is denied. There is no permissive mode, no fallback to a shell.

### Three layers of defense in depth

| Layer | Mechanism | Protection |
|-------|-----------|------------|
| 1 | `command=` + `restrict` in `authorized_keys` | Forces the access level, blocks forwarding/PTY |
| 2 | SSH-Frontière (login shell) | Validates the command against the TOML whitelist |
| 3 | `sudo` whitelist in sudoers | Restricts privileged system commands |

Even if an attacker compromises an SSH key (layer 1), they can only execute commands authorized in the TOML whitelist (layer 2). Even if they bypass layer 2, they can only escalate privileges for commands authorized in sudoers (layer 3).

### Grammatical parser, not a blacklist

SSH-Frontière **is not a shell**. Security does not rely on character filtering (no blacklist of `|`, `;`, `&`), but on a **grammatical parser**.

The expected grammar is `domain action [key=value ...]`. Anything that does not match this structure is rejected. Special characters inside quotes are argument content, not syntax — they are valid.

`std::process::Command` executes directly, without going through an intermediate shell. Command injection is **structurally impossible**.

### Determinism against AI agents

This behavior is **deterministic**: a given command always produces the same validation result, regardless of context. This is an essential property when working with AI agents, whose nature is precisely **indeterminism** — a model can be biased, or the agent's production chain can be corrupted, targeting shells to extract additional information or exfiltrate secrets. With SSH-Frontière, a compromised agent cannot bypass the whitelist, cannot inject commands into a shell, and cannot access unconfigured resources. This is **structurally impossible**.

## What has been implemented

### Rust language

SSH-Frontière is written in Rust, which eliminates the most common vulnerability classes in system programs:
- No buffer overflow
- No use-after-free
- No null pointer dereference
- No `unsafe` in the code (forbidden by the lint configuration in `Cargo.toml`: `unsafe_code = "deny"`)

### 399 cargo tests + 72 E2E SSH scenarios

The project is covered by **399 cargo tests** and **72 additional E2E SSH scenarios**:

| Type | Count | Description |
|------|-------|-------------|
| Unit tests | ~340 | Each module tested independently (10 `*_tests.rs` files) |
| Integration tests | 50 | Complete stdio scenarios (binary execution) |
| Conformance tests | 1 (6 scenarios) | JSON interface contract validation (ADR 0003) |
| Proptest tests | 8 | Property-based tests (constraint-guided fuzzing) |
| **Cargo total** | **399** | |
| E2E SSH scenarios | 72 | Docker Compose with real SSH server |
| cargo-fuzz harnesses | 9 | Unguided fuzzing (random mutations) |

The E2E SSH tests cover the complete protocol, authentication, sessions, security, robustness, and logging. They run in a Docker Compose environment with a real SSH server.

### Dependency auditing

- `cargo deny` in CI: checks licenses and known vulnerabilities (RustSec database)
- `cargo audit`: dependency security audit
- `cargo clippy` in pedantic mode: 0 warnings allowed
- Only 3 direct dependencies: `serde`, `serde_json`, `toml` — all widely audited by the Rust community

### RBAC access control

Three hierarchical trust levels:

| Level | Usage | Examples |
|-------|-------|----------|
| `read` | Read-only | healthcheck, status, list |
| `ops` | Routine operations | backup, deploy, restart |
| `admin` | All actions | configuration, sensitive data |

Each action has a required level. Each SSH connection has an effective level (via `--level` in `authorized_keys` or via token authentication).

### Visibility tags

In addition to vertical RBAC, **tags** enable horizontal filtering: a token with the `forgejo` tag only sees actions tagged `forgejo`, even if it has the `ops` level.

### Token authentication

Two authentication modes:

- **Simple mode** (`challenge_nonce = false`): challenge-response `SHA-256(secret)` — the client proves it knows the secret
- **Nonce mode** (`challenge_nonce = true`): challenge-response `SHA-256(XOR_encrypt(secret || nonce, secret))` with the nonce sent by the server. The nonce is regenerated after each successful authentication, preventing replay of an intercepted proof

### Additional protections

- **Timeout** per command with process group kill (SIGTERM then SIGKILL)
- **Lockout** after N failed authentication attempts (configurable, default: 3)
- **IP ban** optional via configurable external command
- **Masking** of sensitive arguments in logs (SHA-256)
- **Size limit** on captured output (stdout, stderr)
- **Environment cleanup**: `env_clear()` on child processes, only `PATH` and `SSH_FRONTIERE_SESSION` are injected

## What is not guaranteed

No software is perfect. Here are the known and documented limitations:

### 8-bit XOR counter

The cryptographic implementation uses an XOR counter with a keystream limited to 8192 bytes. This is sufficient for current usage (64-character SHA-256 proofs), but not designed for encrypting large volumes.

### Length leak in comparison

The constant-time comparison may reveal the length of compared values. In practice, SHA-256 proofs are always 64 characters, making this leak negligible.

### Per-connection rate limiting

The authentication attempt counter is local to each SSH connection. An attacker can open N connections and have N × `max_auth_failures` attempts. Recommendation: combine with fail2ban, `sshd MaxAuthTries`, or iptables rules.

### Reporting a vulnerability

**Do not report vulnerabilities via public issues.** Contact the maintainer directly for responsible disclosure. The process is described in the [contribution guide](@/contribuer.md).

## Dependencies

SSH-Frontière has a strict policy of minimal dependencies. Each external crate is evaluated against a weighted matrix (license, governance, community, size, transitive dependencies).

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| `serde` | 1.x | Serialization/deserialization | Rust de facto standard, required for JSON and TOML |
| `serde_json` | 1.x | JSON responses | Protocol output format |
| `toml` | 0.8.x | Configuration loading | Rust standard for configuration |

Dev dependency: `proptest` (property tests only, not in the final binary).

Authorized sources: **crates.io only**. No external git repository allowed. Policy verified by `cargo deny`.
