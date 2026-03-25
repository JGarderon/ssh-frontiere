# SSH Frontière v3.0.0

**Website**: [pages.nothus.fr/ssh-frontiere](https://pages.nothus.fr/ssh-frontiere/de/)

[![License: EUPL-1.2](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.md)

Eingeschränkte SSH-Login-Shell in Rust — ein einziger, sicherer Einstiegspunkt für alle eingehenden SSH-Verbindungen auf einem Server.

SSH Frontière ersetzt die Standard-Shell (`/bin/bash`) in `/etc/passwd` und fungiert als **sicherer Dispatcher**: Es validiert jeden SSH-Befehl gegen eine TOML-Whitelist, erzwingt dreistufige RBAC-Zugriffskontrolle und gibt Ergebnisse als strukturiertes JSON über ein Header-basiertes Protokoll auf stdin/stdout zurück.

## Zweck

SSH Frontière ist eine **Sicherheitskomponente** für SSH-Dienstkonten:

- **CI/CD-Runner** (Forgejo Actions, GitHub Actions): Infrastrukturoperationen aus Containern
- **KI-Agenten** (Claude Code, etc.): kontrollierter Serverzugriff mit Vertrauensstufen
- **Automatisierte Wartung**: Backups, Deployments, Healthchecks

Das Programm ist **synchron und one-shot**: SSH erstellt für jede Verbindung einen neuen Prozess, der Dispatcher validiert und führt aus, dann beendet er sich. Kein Daemon, kein Async, kein Tokio.

## Installation

### Voraussetzungen

- Rust 1.70+ mit dem Target `x86_64-unknown-linux-musl`
- `make` (optional, für Abkürzungen)

### Kompilierung

```bash
# Via make
make release

# Oder direkt
cargo build --release --target x86_64-unknown-linux-musl
```

Das resultierende statische Binär (`target/x86_64-unknown-linux-musl/release/ssh-frontiere`, ~1-2 MB) kann ohne Systemabhängigkeiten deployt werden.

```bash
sudo cp target/x86_64-unknown-linux-musl/release/ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere
```

## TOML-Konfiguration

Standarddatei: `/etc/ssh-frontiere/config.toml`.
Überschreiben: `--config <pfad>` oder Umgebungsvariable `SSH_FRONTIERE_CONFIG`.

### Vollständiges Beispiel

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300          # Standardtimeout (Sekunden)
default_level = "read"         # Standard-RBAC-Stufe
mask_sensitive = true           # Sensible Argumente in Logs maskieren
max_stdout_chars = 65536       # Limit für erfassten stdout
max_stderr_chars = 16384       # Limit für erfassten stderr
max_output_chars = 131072      # Globales Hartlimit
timeout_session = 3600         # Session-Keepalive-Timeout (Sekunden)
max_auth_failures = 3          # Auth-Versuche vor Sperrung
log_comments = false           # Kommentare des Clients protokollieren
ban_command = ""               # IP-Sperr-Befehl (z. B. "/usr/sbin/iptables -A INPUT -s {ip} -j DROP")

# --- RBAC-Authentifizierung (optional) ---

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Base64-kodiertes Secret mit b64:-Präfix
level = "ops"                                # Durch dieses Token vergebene Stufe

[auth.tokens.admin-deploy]
secret = "b64:c2VjcmV0LWFkbWluLWRlcGxveQ=="
level = "admin"

# --- Domains und Aktionen ---

[domains.forgejo]
description = "Git-Forge-Infrastruktur"

[domains.forgejo.actions.backup-config]
description = "Forgejo-Konfiguration sichern"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
args = []

[domains.forgejo.actions.deploy]
description = "Eine Version deployen"
level = "admin"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {version}"

[[domains.forgejo.actions.deploy.args]]
name = "version"
type = "enum"
values = ["latest", "stable", "canary"]

[domains.infra]
description = "Server-Infrastruktur"

[domains.infra.actions.healthcheck]
description = "Healthcheck"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"
args = []

[domains.infra.actions.set-password]
description = "Dienstpasswort ändern"
level = "admin"
timeout = 30
execute = "sudo /usr/local/bin/set-password.sh {password}"

[[domains.infra.actions.set-password.args]]
name = "password"
type = "string"
sensitive = true    # Bei mask_sensitive = true in Logs maskiert
```

### Argumenttypen

| Typ | Beschreibung | Validierung |
|-----|-------------|-------------|
| `string` | Freier Text | Max. 256 Zeichen |
| `enum` | Wert aus einer Liste | Muss einem Wert in `values` entsprechen |

### Platzhalter in `execute`

- `{domain}`: wird durch den Domain-Namen ersetzt (immer verfügbar)
- `{arg_name}`: wird durch den entsprechenden Argumentwert ersetzt

## Deployment

### 1. Login-Shell (`/etc/passwd`)

```bash
# Dienstkonto erstellen
sudo useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

Das Programm wird direkt von `sshd` als Login-Shell aufgerufen.

### 2. SSH-Schlüssel mit `authorized_keys`

```
# ~forge-runner/.ssh/authorized_keys

# CI-Runner-Schlüssel (ops-Stufe)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-key

# Monitoring-Schlüssel (nur lesende Stufe)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitor-key

# Admin-Schlüssel (Admin-Stufe)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

Die Option `command=` erzwingt die Ausführung von ssh-frontiere mit der gewählten `--level`, unabhängig vom vom Client gesendeten Befehl. Die Option `restrict` deaktiviert Port-Forwarding, Agent-Forwarding, PTY und X11.

### 3. Sudoers (Schicht 3)

```
# /etc/sudoers.d/ssh-frontiere
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/set-password.sh
```

Nur in der TOML-Whitelist gelistete **und** in sudoers autorisierte Befehle können mit erhöhten Rechten ausgeführt werden.

## Header-Protokoll

SSH Frontière verwendet ein Textprotokoll auf stdin/stdout mit 4 Präfixen (ADR 0006).

### Präfixe

| Präfix | Rolle | Richtung |
|--------|------|----------|
| `+` | **Konfigurieren**: Direktiven (`capabilities`, `challenge`, `auth`, `session`) | bidirektional |
| `#` | **Kommentar**: Informationen, Banner, Nachrichten | bidirektional |
| `$` | **Befehl**: auszuführender Befehl | Client → Server |
| `>` | **Antworten**: JSON-Antwort | Server → Client |

### Verbindungsablauf

```
CLIENT                              SERVER
  |                                    |
  |  <-- Banner + Capabilities -----  |   # ssh-frontiere 3.0.0
  |  <-- Challenge-Nonce -----------  |   + capabilities rbac, session, help
  |                                    |   + challenge nonce=a1b2c3...
  |                                    |   # type "help" for available commands
  |                                    |
  |  --- +auth (optional) -------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (optional) ----->   |   + session keepalive
  |  --- # Kommentar (opt.) ------->  |   # client-id: forgejo-runner-12
  |  --- Leerzeile ---------------->  |   (Ende der Header)
  |                                    |
  |  --- Domain Aktion [Args] ----->  |   forgejo backup-config
  |  --- . ------------------------>  |   . (Ende des Befehlsblocks)
  |  <-- >> stdout (Streaming) -----  |   >> Backup completed
  |  <-- >>> JSON-Antwort ----------  |   >>> {"command":"forgejo backup-config","status_code":0,...}
  |                                    |
```

### JSON-Antwort (4 Felder)

Jeder Befehl erzeugt eine `>>>`-Antwort mit einem JSON-Objekt:

```json
{
  "command": "forgejo backup-config",
  "status_code": 0,
  "status_message": "executed",
  "stdout": null,
  "stderr": null
}
```

- `stdout`/`stderr` = `null`: Ausgabe wurde via `>>` / `>>!`-Präfixe gestreamt
- `status_code` = 0: Erfolg (Exit-Code des Kindprozesses im Passthrough)

### Exit-Codes

| Code | Bedeutung |
|------|-----------|
| 0 | Erfolg |
| 1-127 | Exit-Code des Kindprozesses (Passthrough) |
| 128 | Befehl abgelehnt |
| 129 | Konfigurationsfehler |
| 130 | Timeout |
| 131 | Unzureichende RBAC-Stufe |
| 132 | Protokollfehler |
| 133 | Body-stdin vorzeitig geschlossen |

## Konkrete Beispiele

### One-Shot-Modus

```bash
# Einfache Pipe:
{
  echo "infra healthcheck"
  echo "."
} | ssh forge-runner@server
```

### Session-Modus (Keepalive)

Der Session-Modus ermöglicht das Senden mehrerer Befehle über eine einzelne SSH-Verbindung:

```bash
{
  echo "+ session keepalive"
  echo "infra healthcheck"
  echo "."
  echo "forgejo backup-config"
  echo "."
  echo "exit"
  echo "."
} | ssh forge-runner@server
```

Der Server antwortet für jeden Befehl mit einer `>>>`-JSON-Zeile.

### RBAC-Authentifizierung (Stufenerhöhung)

Ein Client mit `--level=read` kann sich per Challenge-Response auf `ops` oder `admin` erhöhen:

```bash
{
  echo "+ auth token=runner-ci proof=<sha256-hex>"
  echo "forgejo backup-config"    # Erfordert ops, über Token autorisiert
  echo "."
} | ssh forge-runner@server
```

Der `proof` ist `SHA-256(secret)` wenn `challenge_nonce = false`, oder `SHA-256(XOR(secret || nonce, secret))` wenn `challenge_nonce = true`. Die effektive Stufe ist `max(--level, token.level)`.

### Entdeckung (help / list)

```bash
# Vollständige Liste zugänglicher Befehle
{ echo "help"; echo "."; } | ssh forge-runner@server

# Domain-Details
{ echo "help forgejo"; echo "."; } | ssh forge-runner@server

# Kurzliste (Domain + Aktion + Beschreibung, JSON)
{ echo "list"; echo "."; } | ssh forge-runner@server
```

Die Befehle `help` und `list` zeigen nur Aktionen, die auf der effektiven Stufe des Clients zugänglich sind.

## Sicherheit

### Drei Verteidigungsschichten

| Schicht | Mechanismus | Schutz |
|---------|-------------|--------|
| 1 | `command=` + `restrict` in `authorized_keys` | Erzwingt `--level`, blockiert Forwarding/PTY |
| 2 | `ssh-frontiere` (Login-Shell) | Validiert Befehl gegen TOML-Whitelist |
| 3 | `sudo`-Whitelist in sudoers | Schränkt privilegierte Systembefehle ein |

Selbst wenn ein Angreifer Schicht 1 umgeht (kompromittierter Schlüssel), blockiert Schicht 2 jeden Befehl außerhalb der Whitelist. Schicht 3 begrenzt die Systemrechte.

### Grammatikalischer Parser, keine Blacklist

**ssh-frontiere ist keine Shell.** Die Sicherheit basiert auf einem **grammatikalischen Parser**, nicht auf Zeichenfilterung.

- Die erwartete Grammatik ist `domain action [args]` — alles, was nicht dieser Struktur entspricht, wird abgelehnt
- Sonderzeichen (`|`, `;`, `&`, `$`, etc.) in Anführungszeichen sind Argument-**Inhalt**, keine Shell-Syntax — sie sind gültig
- Es gibt keine „verbotenen Zeichen" — es gibt eine Grammatik, und alles, was ihr nicht entspricht, wird abgelehnt
- `std::process::Command` führt direkt ohne Shell-Vermittler aus — Injection ist strukturell unmöglich

### Was das Programm NIE tut

- Eine Shell aufrufen (`/bin/bash`, `/bin/sh`)
- Pipes, Umleitungen oder Verkettungen akzeptieren (`|`, `>`, `&&`, `;`)
- Einen Befehl ausführen, der nicht in der Whitelist steht
- Zugriff auf ein interaktives TTY gewähren

### Zusätzliche Schutzmaßnahmen

- **Timeout** pro Befehl mit Prozessgruppen-Kill (SIGTERM, dann SIGKILL)
- **Sperrung** nach N fehlgeschlagenen Auth-Versuchen (konfigurierbar, Standard: 3)
- **IP-Sperre** optional über konfigurierbaren externen Befehl (`ban_command`)
- **Maskierung** sensibler Argumente in JSON-Logs
- **Größenbeschränkungen** für erfasste Ausgaben (stdout, stderr)
- **Anti-Replay-Nonce** nach jeder erfolgreichen Session-Authentifizierung neu generiert
- **env_clear()** bei Kindprozessen (nur `PATH` bleibt erhalten)

## Tests

```bash
# Unit- und Integrationstests
make test

# End-to-End-SSH-Tests (Docker erforderlich)
make e2e

# Lints (fmt + clippy)
make lint

# Sicherheitsaudit der Abhängigkeiten
make audit
```

E2E-Tests (`make e2e`) starten eine Docker Compose-Umgebung mit SSH-Server und -Client und führen Szenarien durch, die das Protokoll (PRO-*), Authentifizierung (AUT-*), Sessions (SES-*), Sicherheit (SEC-*), Robustheit (ROB-*) und Logging (LOG-*) abdecken.

## Mitwirken

Beiträge sind willkommen! Einzelheiten finden Sie im [Mitwirkungshandbuch](CONTRIBUTING.md).

## Lizenz

Dieses Projekt wird unter der [Öffentlichen Lizenz der Europäischen Union (EUPL-1.2)](LICENSE.md) vertrieben.

Copyright (c) Julien Garderon, 2024-2026
