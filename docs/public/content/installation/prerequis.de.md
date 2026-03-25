+++
title = "Voraussetzungen"
description = "Was Sie für die Installation von SSH-Frontière brauchen"
date = 2026-03-24
weight = 1
+++

# Voraussetzungen

## Zielserver

| Element | Detail |
|---------|--------|
| System | Linux x86_64 |
| SSH-Zugang | Funktionierender `sshd` |
| Dienstkonto | Ein dedizierter Benutzer (z.B. `forge-runner`) |
| Rettungs-Admin-Konto | Ein Konto mit `/bin/bash` (wird nie geändert) |
| Konsolenzugang | IPMI, KVM oder Cloud-Konsole — bei SSH-Lockout |

**Wichtig**: Behalten Sie immer einen funktionierenden Konsolenzugang und ein Admin-Konto mit regulärer Shell. Wenn die SSH-Frontière-Login-Shell falsch konfiguriert ist, könnten Sie den SSH-Zugang zum Dienstkonto verlieren.

## Build-Maschine

Um SSH-Frontière aus den Quellen zu kompilieren:

| Element | Detail |
|---------|--------|
| Rust | Version 1.70 oder höher |
| musl-Target | `x86_64-unknown-linux-musl` (für ein statisches Binary) |
| `make` | Optional, für Makefile-Shortcuts |

### musl-Target installieren

```bash
rustup target add x86_64-unknown-linux-musl
```

## Alternative: vorkompiliertes Binary

Wenn Sie nicht kompilieren möchten, können Sie das statische Binary von den [Projekt-Releases](https://github.com/nothus-forge/ssh-frontiere/releases) herunterladen. Das Binary hat keine Systemabhängigkeit.

---

**Weiter**: [Kompilierung aus den Quellen](@/installation/compilation.md)
