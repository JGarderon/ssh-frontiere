+++
title = "SSH-Frontière"
description = "Restricted SSH login shell in Rust — declarative control of incoming connections"
sort_by = "weight"

[extra]
framed = true
+++

# SSH-Frontière

**Restricted SSH login shell in Rust** — a single, secure entry point for all incoming SSH connections.

SSH-Frontière replaces a Unix account's default shell (`/bin/bash`) with a program that **validates every command** against a declarative TOML configuration before executing it.

[![GitHub](https://img.shields.io/badge/GitHub-Open--source_repository-181717?logo=github&logoColor=white&style=for-the-badge)](https://github.com/JGarderon/ssh-frontiere)

---

## Why SSH-Frontière?

**Secure by default** — No command runs without being explicitly authorized. Deny by default, no shell, no injection possible.

**Simple to deploy** — A ~1 MB static binary, one TOML file, one line in `/etc/passwd`. No daemon, no service to manage.

**Flexible** — Three access levels (read, ops, admin), visibility tags, a structured header protocol. Compatible with AI agents, CI/CD runners, and maintenance scripts.

**Auditable** — Every command executed or denied is logged in structured JSON. 399 cargo tests + 72 E2E SSH scenarios.

---

## Use cases

- **CI/CD runners** (Forgejo Actions, GitHub Actions): deployments, backups, health checks via SSH
- **AI agents** (Claude Code, etc.): controlled access to server resources with trust levels
- **Automated maintenance**: backup, monitoring, and notification scripts

---

## At a glance

| | |
|---|---|
| **Language** | Rust (static musl binary, ~1 MB) |
| **License** | [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12) — European Union Public License |
| **Tests** | 399 cargo + 72 E2E SSH + 9 fuzz harnesses |
| **Dependencies** | 3 direct crates (`serde`, `serde_json`, `toml`) |
| **Configuration** | Declarative TOML |
| **Protocol** | Text headers over stdin/stdout, JSON responses |

---

## Getting started

- [Discover SSH-Frontière](@/presentation.md) — what it is, what it does, why it exists
- [Installation](@/installation/_index.md) — compile, configure, deploy
- [Guides](@/guides/_index.md) — step-by-step tutorials
- [Security](@/securite.md) — security model and guarantees
- [Architecture](@/architecture.md) — technical design
- [Alternatives](@/alternatives.md) — comparison with other solutions
- [FAQ](@/faq.md) — frequently asked questions
- [Contribute](@/contribuer.md) — participate in the project
