+++
title = "SSH-Frontière"
description = "Eingeschränkte SSH-Login-Shell in Rust — deklarative Kontrolle eingehender Verbindungen"
sort_by = "weight"

[extra]
framed = true
+++

# SSH-Frontière

**Eingeschränkte SSH-Login-Shell in Rust** — ein einziger, sicherer Einstiegspunkt für alle eingehenden SSH-Verbindungen.

SSH-Frontière ersetzt die Standard-Shell eines Unix-Kontos (`/bin/bash`) durch ein Programm, das **jeden Befehl** gegen eine deklarative TOML-Konfiguration prüft, bevor er ausgeführt wird.

[![GitHub](https://img.shields.io/badge/GitHub-Open--Source--Repository-181717?logo=github&logoColor=white&style=for-the-badge)](https://github.com/JGarderon/ssh-frontiere)

---

## Warum SSH-Frontière?

**Standardmäßig sicher** — Kein Befehl wird ohne ausdrückliche Genehmigung ausgeführt. Deny by Default, keine Shell, keine Injection möglich.

**Einfach zu deployen** — Ein statisches Binary von ~1 MB, eine TOML-Datei, eine Zeile in `/etc/passwd`. Kein Daemon, kein Dienst zu verwalten.

**Flexibel** — Drei Zugriffsebenen (read, ops, admin), Sichtbarkeits-Tags, ein strukturiertes Header-Protokoll. Kompatibel mit KI-Agenten, CI/CD-Runnern und Wartungsskripten.

**Auditierbar** — Jeder ausgeführte oder abgelehnte Befehl wird in strukturiertem JSON protokolliert. 399 Cargo-Tests + 72 E2E-SSH-Szenarien.

---

## Anwendungsfälle

- **CI/CD-Runner** (Forgejo Actions, GitHub Actions): Deployments, Backups, Healthchecks via SSH
- **KI-Agenten** (Claude Code usw.): kontrollierter Zugriff auf Server-Ressourcen mit Vertrauensstufen
- **Automatisierte Wartung**: Backup-, Überwachungs- und Benachrichtigungsskripte

---

## Auf einen Blick

| | |
|---|---|
| **Sprache** | Rust (statisches musl-Binary, ~1 MB) |
| **Lizenz** | [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12) — Europäische Öffentliche Lizenz |
| **Tests** | 399 Cargo + 72 E2E SSH + 9 Fuzz-Harnesses |
| **Abhängigkeiten** | 3 direkte Crates (`serde`, `serde_json`, `toml`) |
| **Konfiguration** | Deklaratives TOML |
| **Protokoll** | Text-Header über stdin/stdout, JSON-Antworten |

---

## Erste Schritte

- [SSH-Frontière entdecken](@/presentation.md) — was es ist, was es tut, warum es existiert
- [Installation](@/installation/_index.md) — kompilieren, konfigurieren, deployen
- [Anleitungen](@/guides/_index.md) — Schritt-für-Schritt-Tutorials
- [Sicherheit](@/securite.md) — Sicherheitsmodell und Garantien
- [Architektur](@/architecture.md) — technisches Design
- [Alternativen](@/alternatives.md) — Vergleich mit anderen Lösungen
- [FAQ](@/faq.md) — häufig gestellte Fragen
- [Mitwirken](@/contribuer.md) — am Projekt teilnehmen
