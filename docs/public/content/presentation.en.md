+++
title = "Overview"
description = "Discover SSH-Frontière: what it is, why it exists, how it works"
date = 2026-03-24
weight = 1
+++

# SSH-Frontière Overview

## The problem

On a Linux server, SSH service accounts (CI runners, AI agents, maintenance scripts) typically use `/bin/bash` as their login shell. This creates several issues:

- **No control**: the SSH client can execute any command
- **No audit trail**: executed commands are not logged in a structured way
- **No granularity**: a script that only needs to read a status has the same privileges as a deployment script

Traditional solutions (`authorized_keys` with `command=`, bash wrapper scripts, SSH bastions) each have their limitations: fragile, hard to audit, or oversized for the need.

## What SSH-Frontière does

SSH-Frontière is a **replacement login shell**. It sits between `sshd` and system commands:

```
SSH Client
    |
    v
sshd (key authentication)
    |
    v
ssh-frontiere (login shell)
    |
    ├── Validates the command against the TOML configuration
    ├── Checks the access level (read / ops / admin)
    ├── Executes the authorized command
    └── Returns the result in structured JSON
```

Each SSH connection creates a new `ssh-frontiere` process that:

1. Displays a banner and server capabilities
2. Reads client headers (authentication, session mode)
3. Reads the command (`domain action [arguments]`, plain text)
4. Validates against the TOML whitelist
5. Executes if authorized, denies otherwise
6. Returns a JSON response and terminates

The program is **synchronous and ephemeral**: no daemon, no service, no persistent state.

## What SSH-Frontière does not do

- **Not an SSH bastion**: no proxy, no connection relay to other servers
- **Not a key manager**: SSH key management stays in `authorized_keys` and `sshd`
- **Not a shell**: no command interpretation, no pipes, no redirection, no interactivity
- **Not a daemon**: runs and dies with each connection

## Concrete use cases

### CI/CD automation

A Forgejo Actions runner deploys an application via SSH:

```bash
# The runner sends the command via SSH
{
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@server
```

SSH-Frontière checks that the runner has the `admin` level, that the `deploy` action exists in the `forgejo` domain, that the `version=stable` argument is an allowed value, then executes the configured deployment script.

### AI agents

A Claude Code agent operates on a server with bounded privileges:

```bash
# The agent discovers available commands
{ echo "list"; echo "."; } | ssh agent-ia@server

# The agent executes a specific action
{ echo "infra healthcheck"; echo "."; } | ssh agent-ia@server
```

The agent only has access to `read`-level actions configured for it. The `help` and `list` commands allow it to discover available actions and their parameters — JSON format, natively parsable.

### Automated maintenance

Cron scripts execute backups via SSH:

```bash
# Nightly backup
{ echo "forgejo backup-config"; echo "."; } | ssh backup@server

# Post-deployment notification
{ echo 'notify send message="Deployment complete"'; echo "."; } | ssh notify@server
```

### Notifications

Trigger notifications (Slack, Olvid, email) as standard SSH-Frontière actions:

```bash
{ echo 'notify slack channel=ops message="Build OK"'; echo "."; } | ssh notify@server
```

## Why SSH-Frontière rather than...

### ...bash scripts in `authorized_keys`?

The `command=` option in `authorized_keys` allows forcing a command, but:
- One script per key — no granularity
- No argument validation
- No access levels
- No structured logging
- The bash script may contain vulnerabilities (injection, globbing)

SSH-Frontière offers declarative configuration, RBAC, JSON logging, and a grammatical parser that eliminates injections.

### ...an SSH bastion (Teleport, Boundary)?

SSH bastions are designed to manage **human** access to servers:
- Heavy to deploy and maintain
- Oversized for service accounts
- Different threat model (interactive user vs automated script)

SSH-Frontière is a lightweight component (~1 MB) designed for **service accounts**: no interactive session, no proxy, just command validation.

### ...`sudo` alone?

`sudo` controls privilege escalation, but:
- Does not control what the SSH client can *request*
- No structured protocol (JSON input/output)
- No integrated logging at the SSH command level

SSH-Frontière and `sudo` are complementary: SSH-Frontière validates the incoming command, `sudo` controls system privileges. These are layers 2 and 3 of the defense in depth.

## The product value

SSH-Frontière brings **declarative governance** to SSH service access:

1. **Everything in one TOML file**: domains, actions, arguments, access levels. No logic scattered across scripts.

2. **Instant deployment**: since the entire configuration is centralized in a single TOML file, deploying a new version is trivial. Each SSH connection creates a new process that re-reads the configuration — changes take effect at the end of the current session or immediately for any new client.

3. **Zero trust by default**: nothing runs without being explicitly configured. No shell, no injection possible.

4. **Auditable**: every attempt (authorized or denied) is logged in structured JSON with timestamp, command, arguments, level, result.

5. **LLM compatible**: AI agents can discover available actions via `help`/`list`, and interact through a structured JSON protocol — no need to parse free text.

6. **European and open source**: EUPL-1.2 license, developed in France, no dependency on a proprietary ecosystem.

---

For more: [Installation](@/installation/_index.md) | [Architecture](@/architecture.md) | [Security](@/securite.md) | [Alternatives](@/alternatives.md)
