+++
title = "Configuration"
description = "Write the SSH-Frontière config.toml file"
date = 2026-03-24
weight = 3
+++

# Configuration

SSH-Frontière uses a TOML file to declare domains, actions, access levels, arguments, and authentication tokens.

## Location

**Default path**: `/etc/ssh-frontiere/config.toml`

**Override** (by priority order):
1. `--config <path>` in the `command=` line of `authorized_keys`
2. Environment variable `SSH_FRONTIERE_CONFIG`
3. Default path

**Recommended permissions**: `root:forge-runner 640` (adjust the group to your service account).

## File structure

```toml
[global]                              # General settings
[domains.<id>]                        # Functional domains
  [domains.<id>.actions.<id>]         # Authorized actions
    [domains.<id>.actions.<id>.args]  # Named arguments (optional)
[auth]                                # RBAC authentication (optional)
  [auth.tokens.<id>]                  # Tokens with secret, level, and tags
```

## `[global]` section

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `log_file` | string | **required** | Path to JSON log file |
| `default_timeout` | integer | `300` | Default timeout in seconds |
| `max_stdout_chars` | integer | `65536` | Stdout limit (64 KB) |
| `max_stderr_chars` | integer | `16384` | Stderr limit (16 KB) |
| `max_output_chars` | integer | `131072` | Global hard limit (128 KB) |
| `max_stream_bytes` | integer | `10485760` | Streaming volume limit (10 MB) |
| `timeout_session` | integer | `3600` | Session keepalive timeout |
| `max_auth_failures` | integer | `3` | Auth attempts before lockout |
| `ban_command` | string | `""` | IP ban command (placeholder `{ip}`) |
| `log_comments` | bool | `false` | Log client `#` lines |
| `expose_session_id` | bool | `false` | Display session UUID in banner |

The keys `log_level`, `default_level`, and `mask_sensitive` are accepted by the parser for backward compatibility with older configurations, but are no longer used.

## `[domains]` section

A **domain** is a functional scope (e.g., `forgejo`, `infra`, `notify`). Each domain contains authorized **actions**.

```toml
[domains.forgejo]
description = "Git forge infrastructure"

[domains.forgejo.actions.backup-config]
description = "Backup the configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
max_body_size = 65536       # Body limit (64 KB, optional)
```

Each action accepts the following keys: `description` (required), `level` (required), `execute` (required), `timeout` (optional, overrides global), `tags` (optional), `max_body_size` (optional, default 65536 bytes — limited for the `+body` protocol).

### Trust levels

Strict hierarchy: `read` < `ops` < `admin`

| Level | Usage |
|-------|-------|
| `read` | Read-only: healthcheck, status, list |
| `ops` | Routine operations: backup, deploy, restart |
| `admin` | All actions + administration |

### Arguments

Arguments are declared as a TOML dictionary:

```toml
[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"enum"` or `"string"` |
| `values` | list | Allowed values (for `enum`) |
| `default` | string | Default value (makes the argument optional) |
| `sensitive` | bool | If `true`, masked in logs |
| `free` | bool | If `true`, accepts any value without constraint |

### Placeholders in `execute`

| Placeholder | Description |
|-------------|-------------|
| `{domain}` | Domain name (always available) |
| `{arg_name}` | Value of the corresponding argument |

### Visibility tags

Tags filter access to actions horizontally. An action without tags is accessible to everyone. An action with tags is only accessible to identities whose tags share at least one tag in common.

```toml
[domains.forgejo.actions.deploy]
# ...
tags = ["forgejo", "deploy"]
```

## `[auth]` section (optional)

RBAC authentication enables privilege escalation via challenge-response:

```toml
[auth]
challenge_nonce = false              # true = anti-replay nonce mode

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Base64-encoded secret
level = "ops"                               # Granted level
tags = ["forgejo"]                          # Visibility tags
```

The secret must be prefixed with `b64:` and base64-encoded. To generate a secret:

```bash
echo -n "my-random-secret" | base64
# bXktcmFuZG9tLXNlY3JldA==
```

## Load-time validation

Configuration is fully validated at each load (fail-fast). On error, the program exits with code 129. Validations:

- Correct TOML syntax
- At least one domain, at least one action per domain
- Each action has a valid `execute` and `level`
- Placeholders `{arg}` in `execute` match declared arguments
- Enum arguments have at least one allowed value
- Default values are in the allowed values list
- `max_stdout_chars` and `max_stderr_chars` <= `max_output_chars`

## Complete example

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300
max_stdout_chars = 65536
max_stderr_chars = 16384
max_output_chars = 131072
timeout_session = 3600
max_auth_failures = 3

[domains.forgejo]
description = "Git forge infrastructure"

[domains.forgejo.actions.backup-config]
description = "Backup the Forgejo configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"

[domains.forgejo.actions.deploy]
description = "Deployment with version tag"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.infra]
description = "Server infrastructure"

[domains.infra.actions.healthcheck]
description = "Service health check"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[auth]
challenge_nonce = false

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

For a detailed guide with all use cases, see the [complete configuration guide](https://github.com/nothus-forge/ssh-frontiere/blob/main/docs/references/configuration.md) in the repository.

---

**Next**: [Deployment](@/installation/deploiement.md) — put into production.
