+++
title = "Alternatives"
description = "Comparison of SSH-Frontière with existing SSH control solutions"
date = 2026-03-24
weight = 4
+++

# Comparison with alternatives

SSH-Frontière is not the only way to control SSH access. This page compares existing approaches to help you choose the right solution.

## Comparison table

| Criterion | `authorized_keys` `command=` | SSH-Frontière | Teleport | Boundary |
|-----------|------------------------------|---------------|----------|----------|
| **Type** | OpenSSH option | Login shell | SSH bastion | SSH bastion |
| **Target** | Single script per key | Service accounts | Human users | Human users |
| **Granularity** | 1 command per key | 3-level RBAC, domains, actions, arguments | Roles, labels, RBAC | IAM policies |
| **Logging** | Unstructured | Structured JSON per command | Full session (replay) | Audit trail |
| **Deployment** | Native (OpenSSH) | 1 binary + 1 TOML file | Cluster (auth server, proxy, node) | Cluster (controller, workers) |
| **Dependencies** | None | 0 system dependencies | Database, certificates | Database |
| **Size** | — | ~1 MB (static binary) | ~100 MB | ~100 MB |
| **Anti-injection** | Script's responsibility | Structural (grammatical parser) | N/A (interactive session) | N/A (interactive session) |
| **LLM compatible** | No | Yes (JSON, help, discovery) | No | No |
| **License** | OpenSSH (BSD) | EUPL-1.2 | AGPL-3.0 (OSS) / Commercial | BSL 1.1 |

## `authorized_keys` with `command=`

The `command=` option in `authorized_keys` forces the execution of a script on each connection. It is the simplest and most widespread solution.

### Advantages

- **Zero installation**: native OpenSSH feature
- **Simple** for a single use case (one key = one command)

### Limitations

- **One script per key**: no fine granularity. For N different actions, you need N keys or a bash script that parses `$SSH_ORIGINAL_COMMAND`
- **No argument validation**: the script receives a raw string and must validate it itself — an injection source if done poorly
- **No access levels**: all keys have the same privileges (or you must code them into the script)
- **No structured logging**: logs depend on the script
- **Fragile**: a bash script with command validation is difficult to secure and maintain

### When to choose `command=`

- Simple need: one SSH key, one fixed command, no parameters
- No audit or RBAC requirement

## Teleport

[Teleport](https://goteleport.com/) is a full SSH bastion with session recording, SSO, certificates, and audit trail.

### Advantages

- **Session recording**: complete replay of every SSH session
- **Integrated SSO**: GitHub, OIDC, SAML
- **Certificates**: no SSH key management
- **Complete audit**: who connected, when, from where, what was done

### Limitations

- **Complex to deploy**: auth server, proxy, node agent, database, certificates
- **Designed for humans**: interactive sessions, no machine-to-machine protocol
- **Oversized** for service accounts: a CI runner doesn't need session recording or SSO
- **Dual license**: the community version (AGPL-3.0) has functional limitations

### When to choose Teleport

- Managing **human** access to a server fleet
- Need for session recording and SSO
- Infrastructure with resources to deploy and maintain a cluster

## HashiCorp Boundary

[Boundary](https://www.boundaryproject.io/) is an access proxy that abstracts connection details and integrates external identity sources.

### Advantages

- **Infrastructure abstraction**: users connect to logical targets, not IPs
- **IAM integration**: Active Directory, OIDC, LDAP
- **Credential injection**: secrets are dynamically injected, never shared

### Limitations

- **Complex**: controller, workers, database, IAM integration
- **Human-oriented**: not designed for automated scripts
- **BSL 1.1 license**: commercial restrictions on the community edition
- **No command-level control**: Boundary controls access to a host, not to a specific command

### When to choose Boundary

- Large server fleet with centralized identity management
- Need for infrastructure abstraction (users don't know the IPs)
- Team with HashiCorp expertise (Vault, Terraform, etc.)

## `sudo` alone

`sudo` controls privilege escalation for system commands. Often used alone to restrict service account actions.

### Advantages

- **Native**: present on all Linux systems
- **Granular**: fine rules per user, command, and arguments

### Limitations

- **Does not control SSH input**: any command can be **requested** via SSH, even if `sudo` blocks escalation
- **No protocol**: no structured response, no integrated JSON logging
- **Complex configuration**: sudoers rules become hard to maintain with many commands

### When to choose `sudo` alone

- Simple environment where risk is low
- SSH input is already controlled by another mechanism (bastion, VPN)

## When to choose SSH-Frontière

SSH-Frontière is designed for a **specific use case**: controlling what service accounts (not humans) can do via SSH.

Choose SSH-Frontière if:

- Your SSH connections are **automated scripts** (CI/CD, AI agents, cron)
- You need **granularity**: domains, actions, arguments, access levels
- You want **structured JSON logging** for audit and observability
- You want **simple deployment**: one binary, one TOML file
- You need **LLM compatibility**: JSON responses, discovery via `help`/`list`
- You don't want to deploy and maintain a cluster (Teleport, Boundary)

Don't choose SSH-Frontière if:

- Your users are **humans** who need rich and complete interactive sessions
- You need an **SSH proxy** to other servers
- You need **SSO**
