# SSH Frontière v3.0.0

**Website**: [pages.nothus.fr/ssh-frontiere](https://pages.nothus.fr/ssh-frontiere/en/)

[![License: EUPL-1.2](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.md)

Restricted SSH login shell written in Rust — a single, secure entry point for all incoming SSH connections on a server.

SSH Frontière replaces the default shell (`/bin/bash`) in `/etc/passwd` and acts as a **secure dispatcher**: it validates every SSH command against a TOML whitelist, enforces 3-level RBAC access control, and returns results as structured JSON through a header-based protocol on stdin/stdout.

## Purpose

SSH Frontière is a **security component** designed for SSH service accounts:

- **CI/CD Runners** (Forgejo Actions, GitHub Actions): infrastructure operations from containers
- **AI Agents** (Claude Code, etc.): controlled server access with trust levels
- **Automated Maintenance**: backups, deployments, healthchecks

The program is **synchronous and one-shot**: SSH creates a new process for each connection, the dispatcher validates and executes, then exits. No daemon, no async, no Tokio.

## Installation

### Prerequisites

- Rust 1.70+ with the `x86_64-unknown-linux-musl` target
- `make` (optional, for shortcuts)

### Compilation

```bash
# Via make
make release

# Or directly
cargo build --release --target x86_64-unknown-linux-musl
```

The resulting static binary (`target/x86_64-unknown-linux-musl/release/ssh-frontiere`, ~1-2 MB) can be deployed without any system dependencies.

```bash
sudo cp target/x86_64-unknown-linux-musl/release/ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere
```

## TOML Configuration

Default file: `/etc/ssh-frontiere/config.toml`.
Override: `--config <path>` or `SSH_FRONTIERE_CONFIG` environment variable.

### Full example

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300          # Default timeout (seconds)
default_level = "read"         # Default RBAC level
mask_sensitive = true           # Mask sensitive arguments in logs
max_stdout_chars = 65536       # Captured stdout limit
max_stderr_chars = 16384       # Captured stderr limit
max_output_chars = 131072      # Global hard limit
timeout_session = 3600         # Session keepalive timeout (seconds)
max_auth_failures = 3          # Auth attempts before lockout
log_comments = false           # Log client comments
ban_command = ""               # IP ban command (e.g., "/usr/sbin/iptables -A INPUT -s {ip} -j DROP")

# --- RBAC Authentication (optional) ---

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Base64-encoded secret with b64: prefix
level = "ops"                                # Level granted by this token

[auth.tokens.admin-deploy]
secret = "b64:c2VjcmV0LWFkbWluLWRlcGxveQ=="
level = "admin"

# --- Domains and actions ---

[domains.forgejo]
description = "Git forge infrastructure"

[domains.forgejo.actions.backup-config]
description = "Back up Forgejo configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
args = []

[domains.forgejo.actions.deploy]
description = "Deploy a version"
level = "admin"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {version}"

[[domains.forgejo.actions.deploy.args]]
name = "version"
type = "enum"
values = ["latest", "stable", "canary"]

[domains.infra]
description = "Server infrastructure"

[domains.infra.actions.healthcheck]
description = "Health check"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"
args = []

[domains.infra.actions.set-password]
description = "Change service password"
level = "admin"
timeout = 30
execute = "sudo /usr/local/bin/set-password.sh {password}"

[[domains.infra.actions.set-password.args]]
name = "password"
type = "string"
sensitive = true    # Masked in logs when mask_sensitive = true
```

### Argument types

| Type | Description | Validation |
|------|-------------|------------|
| `string` | Free text | Max 256 characters |
| `enum` | Value from a list | Must match a value in `values` |

### Placeholders in `execute`

- `{domain}`: replaced with the domain name (always available)
- `{arg_name}`: replaced with the corresponding argument value

## Deployment

### 1. Login shell (`/etc/passwd`)

```bash
# Create the service account
sudo useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

The program will be invoked directly by `sshd` as a login shell.

### 2. SSH keys with `authorized_keys`

```
# ~forge-runner/.ssh/authorized_keys

# CI runner key (ops level)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-key

# Monitoring key (read-only level)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitor-key

# Admin key (admin level)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

The `command=` option forces ssh-frontiere execution with the chosen `--level`, regardless of the command sent by the client. The `restrict` option disables port forwarding, agent forwarding, PTY, and X11.

### 3. Sudoers (layer 3)

```
# /etc/sudoers.d/ssh-frontiere
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/set-password.sh
```

Only commands listed in the TOML whitelist **and** authorized in sudoers can execute with elevated privileges.

## Header Protocol

SSH Frontière uses a text protocol on stdin/stdout with 4 prefixes (ADR 0006).

### Prefixes

| Prefix | Role | Direction |
|--------|------|-----------|
| `+` | **Configure**: directives (`capabilities`, `challenge`, `auth`, `session`) | bidirectional |
| `#` | **Comment**: information, banner, messages | bidirectional |
| `$` | **Command**: command to execute | client -> server |
| `>` | **Respond**: JSON response | server -> client |

### Connection flow

```
CLIENT                              SERVER
  |                                    |
  |  <-- banner + capabilities -----  |   # ssh-frontiere 3.0.0
  |  <-- challenge nonce -----------  |   + capabilities rbac, session, help
  |                                    |   + challenge nonce=a1b2c3...
  |                                    |   # type "help" for available commands
  |                                    |
  |  --- +auth (optional) -------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (optional) ----->   |   + session keepalive
  |  --- # comment (opt.) -------->   |   # client-id: forgejo-runner-12
  |  --- empty line --------------->   |   (end of headers)
  |                                    |
  |  --- domain action [args] ----->   |   forgejo backup-config
  |  --- . ------------------------>   |   . (end of command block)
  |  <-- >> stdout (streaming) -----  |   >> Backup completed
  |  <-- >>> JSON response ---------  |   >>> {"command":"forgejo backup-config","status_code":0,...}
  |                                    |
```

### JSON Response (4 fields)

Each command produces a `>>>` response containing a JSON object:

```json
{
  "command": "forgejo backup-config",
  "status_code": 0,
  "status_message": "executed",
  "stdout": null,
  "stderr": null
}
```

- `stdout`/`stderr` = `null`: output was streamed via `>>` / `>>!` prefixes
- `status_code` = 0: success (child process exit code in passthrough)

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1-127 | Child command exit code (passthrough) |
| 128 | Command rejected |
| 129 | Configuration error |
| 130 | Timeout |
| 131 | Insufficient RBAC level |
| 132 | Protocol error |
| 133 | Body stdin closed prematurely |

## Concrete Examples

### One-shot mode

```bash
# Simple pipe:
{
  echo "infra healthcheck"
  echo "."
} | ssh forge-runner@server
```

### Session mode (keepalive)

Session mode allows sending multiple commands in a single SSH connection:

```bash
{
  echo "+ session keepalive"
  echo "infra healthcheck"
  echo "."
  echo "forgejo backup-config"
  echo "."
  echo "exit"
  echo "."
} | ssh forge-runner@server
```

The server responds with a `>>>` JSON line for each command.

### RBAC Authentication (level elevation)

A client with `--level=read` can elevate to `ops` or `admin` via challenge-response:

```bash
{
  echo "+ auth token=runner-ci proof=<sha256-hex>"
  echo "forgejo backup-config"    # Requires ops, authorized via token
  echo "."
} | ssh forge-runner@server
```

The `proof` is `SHA-256(secret)` when `challenge_nonce = false`, or `SHA-256(XOR(secret || nonce, secret))` when `challenge_nonce = true`. The effective level is `max(--level, token.level)`.

### Discovery (help / list)

```bash
# Full list of accessible commands
{ echo "help"; echo "."; } | ssh forge-runner@server

# Domain details
{ echo "help forgejo"; echo "."; } | ssh forge-runner@server

# Short list (domain + action + description, JSON)
{ echo "list"; echo "."; } | ssh forge-runner@server
```

The `help` and `list` commands only show actions accessible at the client's effective level.

## Security

### Three layers of defense in depth

| Layer | Mechanism | Protection |
|-------|-----------|------------|
| 1 | `command=` + `restrict` in `authorized_keys` | Forces `--level`, blocks forwarding/PTY |
| 2 | `ssh-frontiere` (login shell) | Validates command against TOML whitelist |
| 3 | `sudo` whitelist in sudoers | Restricts privileged system commands |

Even if an attacker bypasses layer 1 (compromised key), layer 2 blocks any command outside the whitelist. Layer 3 limits system privileges.

### Grammatical parser, not a blacklist

**ssh-frontiere is not a shell.** Security relies on a **grammatical parser**, not character filtering.

- The expected grammar is `domain action [args]` — anything not matching this structure is rejected
- Special characters (`|`, `;`, `&`, `$`, etc.) within quotes are argument **content**, not shell syntax — they are valid
- There are no "forbidden characters" — there is a grammar, and anything not matching it is rejected
- `std::process::Command` executes directly without a shell intermediary — injection is structurally impossible

### What the program NEVER does

- Invoke a shell (`/bin/bash`, `/bin/sh`)
- Accept pipes, redirections, or chaining (`|`, `>`, `&&`, `;`)
- Execute a command not listed in the whitelist
- Provide access to an interactive TTY

### Additional protections

- **Timeout** per command with process group kill (SIGTERM then SIGKILL)
- **Lockout** after N failed auth attempts (configurable, default: 3)
- **IP Ban** optional via configurable external command (`ban_command`)
- **Masking** of sensitive arguments in JSON logs
- **Size limits** on captured output (stdout, stderr)
- **Anti-replay nonce** regenerated after each successful session authentication
- **env_clear()** on child processes (only `PATH` is preserved)

## Tests

```bash
# Unit and integration tests
make test

# End-to-end SSH tests (Docker required)
make e2e

# Lints (fmt + clippy)
make lint

# Dependency security audit
make audit
```

E2E tests (`make e2e`) start a Docker Compose environment with an SSH server and client, then run scenarios covering the protocol (PRO-*), authentication (AUT-*), sessions (SES-*), security (SEC-*), robustness (ROB-*), and logging (LOG-*).

## Contributing

Contributions are welcome! See the [contributing guide](CONTRIBUTING.md) for details.

## License

This project is distributed under the [European Union Public License (EUPL-1.2)](LICENSE.md).

Copyright (c) Julien Garderon, 2024-2026
