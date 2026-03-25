+++
title = "Architecture"
description = "Technical design of SSH-Frontière: language, modules, protocol, dependencies"
date = 2026-03-24
weight = 3
+++

# Architecture and design

## Why Rust

SSH-Frontière is written in Rust for three reasons:

1. **Memory safety**: no buffer overflow, no use-after-free, no null pointer. For a security component running as a login shell, this is critical.

2. **Static binary**: compiles with the `x86_64-unknown-linux-musl` target (other targets possible without guaranteed functionality), the binary is ~1 MB and has no system dependency. Copy it to the server and it's ready.

3. **Performance**: the program starts, validates, executes, and dies in milliseconds. No runtime, no garbage collector, no JIT.

## Synchronous and ephemeral

SSH-Frontière is a **synchronous and one-shot** program. No daemon, no async, no Tokio.

The lifecycle is simple:
1. `sshd` authenticates the SSH connection by key
2. `sshd` forks and executes `ssh-frontiere` as the login shell
3. `ssh-frontiere` validates and executes the command
4. The process terminates

Each SSH connection creates a new process. No shared state between connections, no concurrency issues.

## Code structure

The code is organized into modules with clear responsibilities:

| Module | Responsibility |
|--------|----------------|
| `main.rs` | Entry point, argument flattening, orchestrator call |
| `orchestrator.rs` | Main flow: banner, headers, command, response, session loop |
| `config.rs` | TOML configuration structures, fail-fast validation |
| `protocol.rs` | Header protocol: parser, banner, auth, session, body |
| `crypto.rs` | SHA-256 (FIPS 180-4 implementation), base64, nonce, challenge-response |
| `dispatch.rs` | Command parsing (quotes, `key=value`), resolution, RBAC |
| `chain_parser.rs` | Command chain parser (operators `;`, `&`, `\|`) |
| `chain_exec.rs` | Chain execution: strict sequence (`;`), permissive (`&`), fallback (`\|`) |
| `discovery.rs` | `help` and `list` commands: domain and action discovery |
| `logging.rs` | Structured JSON logging, sensitive argument masking |
| `output.rs` | JSON response, exit codes |
| `lib.rs` | Exposes `crypto` for the proof binary and fuzz helpers |

Each module has its test file (`*_tests.rs`) in the same directory.

An auxiliary `proof` binary (`src/bin/proof.rs`) computes authentication proofs for E2E tests and client integration.

## Header protocol

SSH-Frontière uses a text protocol over stdin/stdout. Prefixes differ by direction:

**Client to server (stdin):**

| Prefix | Role |
|--------|------|
| `+ ` | **Configure**: directives (`auth`, `session`, `body`) |
| `# ` | **Comment**: ignored by the server |
| *(plain text)* | **Command**: `domain action [arguments]` |
| `.` *(alone on a line)* | **End of block**: terminates a command block |

**Server to client (stdout):**

| Prefix | Role |
|--------|------|
| `#> ` | **Comment**: banner, informational messages |
| `+> ` | **Configure**: capabilities, challenge nonce |
| `>>> ` | **Response**: final JSON response |
| `>> ` | **Stdout**: standard output streaming (ADR 0011) |
| `>>! ` | **Stderr**: error output streaming |

### Connection flow

```
CLIENT                                  SERVER
  |                                        |
  |  <-- banner + capabilities ----------  |   #> ssh-frontiere 0.1.0
  |                                        |   +> capabilities rbac, session, help, body
  |                                        |   +> challenge nonce=a1b2c3...
  |                                        |   #> type "help" for available commands
  |                                        |
  |  --- +auth (optional) ------------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (optional) ---------->   |   + session keepalive
  |                                        |
  |  --- command (plain text) --------->   |   forgejo backup-config
  |  --- end of block ----------------->   |   .
  |  <-- streaming stdout -------------   |   >> Backup completed
  |  <-- final JSON response ----------   |   >>> {"status_code":0,"status_message":"executed",...}
  |                                        |
  |  (if session keepalive)                |
  |  --- command 2 -------------------->   |   infra healthcheck
  |  --- end of block ----------------->   |   .
  |  <-- JSON response 2 -------------   |   >>> {"status_code":0,...}
  |  --- end session (empty block) --->   |   .
  |  <-- session closed ---------------   |   #> session closed
```

### JSON response

Each command produces a final JSON response on a single line, prefixed by `>>>`. Standard output and errors are sent in streaming via `>>` and `>>!`:

```
>> Backup completed
>>> {"command":"forgejo backup-config","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` in the final JSON response: the output was sent in streaming via `>>` and `>>!`
- For non-executed commands (rejection, config error), `stdout` and `stderr` are also `null`

### Body protocol

The `+body` header allows transmitting multiline content to the child process via stdin. Four delimitation modes:

- `+body`: reads until a line containing only `.` (dot)
- `+body size=N`: reads exactly N bytes
- `+body stop="DELIMITER"`: reads until a line containing the delimiter
- `+body size=N stop="DELIMITER"`: first delimiter reached (size or marker) ends reading

## TOML configuration

The configuration format is declarative TOML. Choice documented in ADR 0001:

- **Why TOML**: human-readable, native typing, Rust ecosystem standard, no significant indentation (unlike YAML), more expressive than JSON for configuration.
- **Why not YAML**: significant indentation error-prone, dangerous implicit types (`on`/`off` → boolean), complex specification.
- **Why not JSON**: no comments, verbose, not designed for human configuration.

Configuration is **validated at load** (fail-fast): TOML syntax, field completeness, placeholder consistency, at least one domain, at least one action per domain, non-empty enum values.

## Dependency policy

SSH-Frontière has a **zero non-essential dependency** policy. Each external crate must be justified by a real need.

### Current dependencies

3 direct dependencies, ~20 transitive dependencies:

| Crate | Usage |
|-------|-------|
| `serde` + `serde_json` | JSON serialization (logging, responses) |
| `toml` | TOML configuration loading |

### Evaluation matrix

Before adding a dependency, it is evaluated on 8 weighted criteria (score /5): license (eliminatory), governance (x3), community (x2), update frequency (x2), size (x3), transitive dependencies (x3), features (x2), non-lock-in (x1). Minimum score: 3.5/5.

### Audit

- `cargo deny` checks licenses and known vulnerabilities
- `cargo audit` searches the RustSec database for flaws
- Authorized sources: crates.io only

## How the project was designed

SSH-Frontière was developed in successive phases (1 to 9, with intermediate phases 2.5 and 5.5), driven by Claude Code agents with systematic TDD methodology:

| Phase | Content |
|-------|---------|
| 1 | Functional dispatcher, TOML config, 3-level RBAC |
| 2 | Production configuration, operations scripts |
| 2.5 | SHA-256 FIPS 180-4, BTreeMap, graceful timeout |
| 3 | Unified header protocol, challenge-response auth, sessions |
| 4 | E2E SSH Docker tests, code cleanup, forge integration |
| 5 | Visibility tags, horizontal token filtering |
| 5.5 | Optional nonce, named arguments, proof binary (includes phase 6, merged) |
| 7 | Configuration guide, dry-run `--check-config`, help without prefix |
| 8 | Structured error types, pedantic clippy, cargo-fuzz, proptest |
| 9 | Body protocol, free arguments, max_body_size, exit code 133 |

The project was designed by:
- **Julien Garderon** (BO): concept, functional specifications, Rust choice, project name
- **Claude supervisor** (PM/Tech Lead): technical analysis, architecture
- **Claude Code agents**: implementation, tests, documentation

Where human and machine work together, better, faster, with greater security.
