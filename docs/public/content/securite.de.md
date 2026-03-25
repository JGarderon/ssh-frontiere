+++
title = "Sicherheit"
description = "Sicherheitsmodell, Garantien und Grenzen von SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Sicherheit

SSH-Frontière ist eine **Sicherheitskomponente**. Ihr Zweck ist es, einzuschränken, was eingehende SSH-Verbindungen tun können. Diese Seite dokumentiert das Sicherheitsmodell, was implementiert wurde und was nicht garantiert wird.

## Sicherheitsmodell

### Grundprinzip: Deny by Default

Nichts wird ausgeführt, ohne explizit konfiguriert zu sein. Wenn ein Befehl nicht in der TOML-Whitelist steht, wird er abgelehnt. Es gibt keinen permissiven Modus, keinen Fallback auf eine Shell.

### Drei Schichten der Tiefenverteidigung

| Schicht | Mechanismus | Schutz |
|---------|-------------|--------|
| 1 | `command=` + `restrict` in `authorized_keys` | Erzwingt die Zugriffsebene, blockiert Forwarding/PTY |
| 2 | SSH-Frontière (Login-Shell) | Validiert den Befehl gegen die TOML-Whitelist |
| 3 | `sudo`-Whitelist in sudoers | Beschränkt privilegierte Systembefehle |

Selbst wenn ein Angreifer einen SSH-Schlüssel kompromittiert (Schicht 1), kann er nur Befehle ausführen, die in der TOML-Whitelist autorisiert sind (Schicht 2). Selbst wenn er Schicht 2 umgeht, kann er Privilegien nur für in sudoers autorisierte Befehle eskalieren (Schicht 3).

### Grammatischer Parser, keine Blacklist

SSH-Frontière **ist keine Shell**. Die Sicherheit basiert nicht auf Zeichenfilterung (keine Blacklist von `|`, `;`, `&`), sondern auf einem **grammatischen Parser**.

Die erwartete Grammatik ist `Domäne Aktion [key=value ...]`. Alles, was nicht dieser Struktur entspricht, wird abgelehnt. Sonderzeichen in Anführungszeichen sind Argumentinhalt, keine Syntax — sie sind gültig.

`std::process::Command` führt direkt aus, ohne eine zwischengeschaltete Shell. Befehlsinjektion ist **strukturell unmöglich**.

### Determinismus gegenüber KI-Agenten

Dieses Verhalten ist **deterministisch**: ein gegebener Befehl erzeugt immer das gleiche Validierungsergebnis, unabhängig vom Kontext. Das ist eine wesentliche Eigenschaft bei der Arbeit mit KI-Agenten, deren Natur gerade der **Indeterminismus** ist — ein Modell kann voreingenommen sein, oder die Produktionskette des Agenten kann kompromittiert sein, um Shells anzugreifen und zusätzliche Informationen zu extrahieren oder Geheimnisse zu exfiltrieren. Mit SSH-Frontière kann ein kompromittierter Agent die Whitelist nicht umgehen, keine Befehle in eine Shell injizieren und nicht auf nicht konfigurierte Ressourcen zugreifen. Das ist **strukturell unmöglich**.

## Was implementiert wurde

### Programmiersprache Rust

SSH-Frontière ist in Rust geschrieben, was die häufigsten Schwachstellenklassen in Systemprogrammen eliminiert:
- Kein Buffer Overflow
- Kein Use-after-free
- Kein Null-Pointer-Dereference
- Kein `unsafe` im Code (verboten durch die Lint-Konfiguration in `Cargo.toml`: `unsafe_code = "deny"`)

### 399 Cargo-Tests + 72 E2E-SSH-Szenarien

Das Projekt ist durch **399 Cargo-Tests** und **72 zusätzliche E2E-SSH-Szenarien** abgedeckt:

| Typ | Anzahl | Beschreibung |
|-----|--------|--------------|
| Unit-Tests | ~340 | Jedes Modul unabhängig getestet (10 `*_tests.rs`-Dateien) |
| Integrationstests | 50 | Komplette stdio-Szenarien (Binary-Ausführung) |
| Konformitätstests | 1 (6 Szenarien) | JSON-Schnittstellenvertrags-Validierung (ADR 0003) |
| Proptest-Tests | 8 | Eigenschaftsbasierte Tests (Constraint-gesteuertes Fuzzing) |
| **Cargo-Gesamt** | **399** | |
| E2E-SSH-Szenarien | 72 | Docker Compose mit echtem SSH-Server |
| cargo-fuzz-Harnesses | 9 | Ungesteuertes Fuzzing (zufällige Mutationen) |

Die E2E-SSH-Tests decken das vollständige Protokoll, Authentifizierung, Sitzungen, Sicherheit, Robustheit und Logging ab. Sie laufen in einer Docker-Compose-Umgebung mit einem echten SSH-Server.

### Abhängigkeits-Audit

- `cargo deny` in CI: prüft Lizenzen und bekannte Schwachstellen (RustSec-Datenbank)
- `cargo audit`: Sicherheits-Audit der Abhängigkeiten
- `cargo clippy` im pedantic-Modus: 0 Warnungen erlaubt
- Nur 3 direkte Abhängigkeiten: `serde`, `serde_json`, `toml` — alle umfassend von der Rust-Community auditiert

### RBAC-Zugriffskontrolle

Drei hierarchische Vertrauensstufen:

| Stufe | Verwendung | Beispiele |
|-------|------------|----------|
| `read` | Nur lesen | healthcheck, status, list |
| `ops` | Routineoperationen | backup, deploy, restart |
| `admin` | Alle Aktionen | Konfiguration, sensible Daten |

Jede Aktion hat eine erforderliche Stufe. Jede SSH-Verbindung hat eine effektive Stufe (über `--level` in `authorized_keys` oder über Token-Authentifizierung).

### Sichtbarkeits-Tags

Zusätzlich zum vertikalen RBAC ermöglichen **Tags** eine horizontale Filterung: ein Token mit dem Tag `forgejo` sieht nur Aktionen mit dem Tag `forgejo`, selbst wenn es die Stufe `ops` hat.

### Token-Authentifizierung

Zwei Authentifizierungsmodi:

- **Einfacher Modus** (`challenge_nonce = false`): Challenge-Response `SHA-256(secret)` — der Client beweist, dass er das Geheimnis kennt
- **Nonce-Modus** (`challenge_nonce = true`): Challenge-Response `SHA-256(XOR_encrypt(secret || nonce, secret))` mit dem vom Server gesendeten Nonce. Der Nonce wird nach jeder erfolgreichen Authentifizierung regeneriert, um das Wiedereinspielen eines abgefangenen Proofs zu verhindern

### Zusätzliche Schutzmaßnahmen

- **Timeout** pro Befehl mit Process-Group-Kill (SIGTERM dann SIGKILL)
- **Lockout** nach N fehlgeschlagenen Authentifizierungsversuchen (konfigurierbar, Standard: 3)
- **IP-Sperre** optional über konfigurierbaren externen Befehl
- **Maskierung** sensibler Argumente in Logs (SHA-256)
- **Größenlimit** für erfasste Ausgabe (stdout, stderr)
- **Umgebungsbereinigung**: `env_clear()` für Kindprozesse, nur `PATH` und `SSH_FRONTIERE_SESSION` werden injiziert

## Was nicht garantiert wird

Keine Software ist perfekt. Hier sind die bekannten und dokumentierten Einschränkungen:

### 8-Bit-XOR-Zähler

Die kryptographische Implementierung verwendet einen XOR-Zähler mit einem auf 8192 Bytes begrenzten Keystream. Das reicht für die aktuelle Nutzung (64-Zeichen-SHA-256-Proofs), ist aber nicht für die Verschlüsselung großer Datenmengen ausgelegt.

### Längenleck beim Vergleich

Der zeitkonstante Vergleich kann die Länge der verglichenen Werte verraten. In der Praxis sind SHA-256-Proofs immer 64 Zeichen lang, was dieses Leck vernachlässigbar macht.

### Rate-Limiting pro Verbindung

Der Zähler für Authentifizierungsversuche ist lokal für jede SSH-Verbindung. Ein Angreifer kann N Verbindungen öffnen und N × `max_auth_failures` Versuche haben. Empfehlung: Kombination mit fail2ban, `sshd MaxAuthTries` oder iptables-Regeln.

### Eine Schwachstelle melden

**Melden Sie Schwachstellen nicht über öffentliche Issues.** Kontaktieren Sie den Maintainer direkt für eine verantwortungsvolle Offenlegung. Der Prozess wird im [Beitragsguide](@/contribuer.md) beschrieben.

## Abhängigkeiten

SSH-Frontière verfolgt eine strikte Politik minimaler Abhängigkeiten. Jede externe Crate wird anhand einer gewichteten Matrix bewertet (Lizenz, Governance, Community, Größe, transitive Abhängigkeiten).

| Crate | Version | Verwendung | Begründung |
|-------|---------|------------|------------|
| `serde` | 1.x | Serialisierung/Deserialisierung | Rust-De-facto-Standard, erforderlich für JSON und TOML |
| `serde_json` | 1.x | JSON-Antworten | Ausgabeformat des Protokolls |
| `toml` | 0.8.x | Konfiguration laden | Rust-Standard für Konfiguration |

Entwicklungsabhängigkeit: `proptest` (nur Eigenschaftstests, nicht im finalen Binary).

Autorisierte Quellen: **nur crates.io**. Kein externes Git-Repository erlaubt. Richtlinie wird von `cargo deny` überprüft.
