+++
title = "FAQ"
description = "Frequently asked questions about SSH-Frontière"
date = 2026-03-24
weight = 5
+++

# Frequently asked questions

## What is SSH-Frontière exactly?

A **replacement login shell** written in Rust. It is installed in place of `/bin/bash` in `/etc/passwd` for a service account. Every SSH connection goes through SSH-Frontière, which validates the command against a TOML configuration file before executing it.

## Is it an SSH bastion?

No. An SSH bastion (Teleport, Boundary) is a **proxy** that relays connections to other servers. SSH-Frontière does not relay — it controls what runs **on the server where it is installed**.

Bastions manage human access to a server fleet. SSH-Frontière manages **service account** access (CI runners, AI agents, scripts) to specific actions on a server.

## Does it replace `sudo`?

No, it's complementary. SSH-Frontière controls what the SSH client **can request** (layer 2). `sudo` controls the system privileges **needed for execution** (layer 3). Both combine for defense in depth.

## Can you use it without a TOML file?

No. The configuration file is mandatory. This is intentional: everything is explicit, declarative, and auditable. No permissive mode, no fallback to a shell.

## What happens if the configuration is invalid?

SSH-Frontière fully validates the configuration at startup (fail-fast). If the configuration is invalid, the program exits with code 129 and an explicit error message in the log. No command is executed. The SSH client only sees that the service is unavailable — **never** the error details. Diagnostic information stays on the server side.

You can test the configuration safely:

```bash
ssh-frontiere --check-config --config /etc/ssh-frontiere/config.toml
```

## How to diagnose a problem?

Several tools are available:

1. **Config validation**: `ssh-frontiere --check-config` checks syntax and consistency
2. **`help` command**: displays actions accessible at the client's effective level
3. **`list` command**: short version (domain + action)
4. **JSON logs**: every command (executed or denied) is logged with timestamp, command, arguments, level, result
5. **Exit code**: 0 = success, 128 = denied, 129 = config error, 130 = timeout, 131 = insufficient level, 132 = protocol error, 133 = body stdin closed prematurely

## Can AI agents use it?

Yes, this is a first-class use case. The `help` and `list` commands return structured JSON, directly parsable by an agent. The header protocol (prefixes `+`, `#`, `$`, `>`) is designed to be machine-readable without disturbing human reading.

See the [AI agents guide](@/guides/agents-ia.md) for detailed configuration.

## What are the source code dependencies?

3 direct dependencies:

| Crate | Usage |
|-------|-------|
| `serde` + `serde_json` | JSON serialization (logs, responses) |
| `toml` | Configuration loading |

No async runtime, no Tokio, no web framework. The static binary is ~1 MB.

## Why Rust and not Go/Python?

1. **Memory safety**: no buffer overflow, no use-after-free — critical for a security component
2. **Static binary**: compiles with musl, no system dependency
3. **Performance**: starts in milliseconds, no runtime
4. **No `unsafe`**: forbidden by Cargo lints (`unsafe_code = "deny"`)

## Why TOML and not YAML or JSON?

- **TOML**: readable, typed, comments, Rust standard, no significant indentation
- **YAML**: significant indentation error-prone, dangerous implicit types (`on`/`off` → boolean)
- **JSON**: no comments, verbose, not designed for human configuration

The choice is documented in ADR 0001.

## How does token authentication work?

Two modes:

1. **Simple mode** (`challenge_nonce = false`): the client computes `SHA-256(secret)` and sends it as proof
2. **Nonce mode** (`challenge_nonce = true`): the server sends a nonce, the client computes `SHA-256(XOR_encrypt(secret || nonce, secret))`

Nonce mode protects against replay: each proof is unique thanks to the nonce.

## Can you use multiple SSH keys?

Yes. Each key in `authorized_keys` has its own `--level`. Multiple keys can coexist with different levels:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin
```

## What is the response format?

Standard output and errors are sent in streaming (prefixes `>>` and `>>!`), then a final JSON response on a single line (prefix `>>>`):

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` in the final JSON: the output was sent in streaming
- `status_code` = 0: success (child process exit code in passthrough)

## How to update SSH-Frontière?

1. Compile the new version (`make release`)
2. Copy the binary to the server (`scp`)
3. Verify (`ssh user@server` + `help`)

No data migration, no database schema. The TOML file is versionable with git.

## How to contribute?

See the [contribution guide](@/contribuer.md). In short: open an issue, fork, TDD, pull request, green CI. AI-generated contributions are accepted.

## Where to find the source code?

The source code is available on the [GitHub repository](https://github.com/nothus-forge/ssh-frontiere). License [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).
