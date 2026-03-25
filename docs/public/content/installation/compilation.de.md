+++
title = "Kompilierung"
description = "SSH-Frontière aus den Quellen kompilieren"
date = 2026-03-24
weight = 2
+++

# Kompilierung aus den Quellen

## Release-Kompilierung

```bash
# Via make (empfohlen)
make release

# Oder direkt mit cargo
cargo build --release --target x86_64-unknown-linux-musl
```

Das resultierende Binary befindet sich unter:

```
target/x86_64-unknown-linux-musl/release/ssh-frontiere
```

Es ist ein **statisches Binary** von ca. 1 MB, ohne Systemabhängigkeit. Es kann direkt auf jeden Linux x86_64 Server kopiert werden.

## Überprüfung

```bash
# Binary-Typ prüfen
file target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ELF 64-bit LSB executable, x86-64, statically linked

# Größe prüfen
ls -lh target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ~1-2 MB
```

## Debug-Kompilierung

Für die Entwicklung:

```bash
make build
# oder
cargo build
```

## Tests

Vor dem Deployment sicherstellen, dass die Tests bestehen:

```bash
# Unit- und Integrationstests
make test

# Lints (Formatierung + Clippy)
make lint

# Abhängigkeits-Audit
make audit
```

## Hilfsbinary: proof

Ein Hilfsbinary ist für die Berechnung von Authentifizierungs-Proofs enthalten:

```bash
cargo build --release --target x86_64-unknown-linux-musl --bin proof
```

Dieses Binary ist nützlich, um die Challenge-Response-Authentifizierung zu testen, ohne die SHA-256-Berechnung clientseitig implementieren zu müssen.

---

**Weiter**: [Konfiguration](@/installation/configuration.md) — die `config.toml`-Datei vorbereiten.
