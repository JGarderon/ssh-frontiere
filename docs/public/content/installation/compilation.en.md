+++
title = "Compilation"
description = "Compile SSH-Frontière from source"
date = 2026-03-24
weight = 2
+++

# Compilation from source

## Release compilation

```bash
# Via make (recommended)
make release

# Or directly with cargo
cargo build --release --target x86_64-unknown-linux-musl
```

The resulting binary is located at:

```
target/x86_64-unknown-linux-musl/release/ssh-frontiere
```

This is a **static binary** of approximately 1 MB, with no system dependency. It can be copied directly to any Linux x86_64 server.

## Verification

```bash
# Check the binary type
file target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ELF 64-bit LSB executable, x86-64, statically linked

# Check the size
ls -lh target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ~1-2 MB
```

## Debug compilation

For development:

```bash
make build
# or
cargo build
```

## Tests

Before deploying, verify that tests pass:

```bash
# Unit and integration tests
make test

# Lints (formatting + clippy)
make lint

# Dependency audit
make audit
```

## Auxiliary binary: proof

An auxiliary binary is included for computing authentication proofs:

```bash
cargo build --release --target x86_64-unknown-linux-musl --bin proof
```

This binary is useful for testing challenge-response authentication without implementing SHA-256 computation on the client side.

---

**Next**: [Configuration](@/installation/configuration.md) — prepare the `config.toml` file.
