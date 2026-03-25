+++
title = "Architektur"
description = "Technisches Design von SSH-Frontière: Sprache, Module, Protokoll, Abhängigkeiten"
date = 2026-03-24
weight = 3
+++

# Architektur und Design

## Warum Rust

SSH-Frontière ist aus drei Gründen in Rust geschrieben:

1. **Speichersicherheit**: kein Buffer Overflow, kein Use-after-free, kein Null-Pointer. Für eine Sicherheitskomponente, die als Login-Shell läuft, ist das entscheidend.

2. **Statisches Binary**: kompiliert mit dem Target `x86_64-unknown-linux-musl` (andere Targets möglich ohne Funktionsgarantie), das Binary ist ~1 MB groß und hat keine Systemabhängigkeit. Auf den Server kopieren und fertig.

3. **Performance**: das Programm startet, validiert, führt aus und beendet sich in Millisekunden. Kein Runtime, kein Garbage Collector, kein JIT.

## Synchron und kurzlebig

SSH-Frontière ist ein **synchrones One-Shot-Programm**. Kein Daemon, kein Async, kein Tokio.

Der Lebenszyklus ist einfach:
1. `sshd` authentifiziert die SSH-Verbindung per Schlüssel
2. `sshd` forkt und führt `ssh-frontiere` als Login-Shell aus
3. `ssh-frontiere` validiert und führt den Befehl aus
4. Der Prozess beendet sich

Jede SSH-Verbindung erzeugt einen neuen Prozess. Kein geteilter Zustand zwischen Verbindungen, keine Nebenläufigkeitsprobleme.

## Codestruktur

Der Code ist in Module mit klaren Verantwortlichkeiten organisiert:

| Modul | Verantwortlichkeit |
|-------|-------------------|
| `main.rs` | Einstiegspunkt, Argument-Flattening, Orchestrator-Aufruf |
| `orchestrator.rs` | Hauptfluss: Banner, Header, Befehl, Antwort, Sitzungsschleife |
| `config.rs` | TOML-Konfigurationsstrukturen, Fail-fast-Validierung |
| `protocol.rs` | Header-Protokoll: Parser, Banner, Auth, Sitzung, Body |
| `crypto.rs` | SHA-256 (FIPS 180-4 Implementierung), Base64, Nonce, Challenge-Response |
| `dispatch.rs` | Befehlsparsing (Anführungszeichen, `key=value`), Auflösung, RBAC |
| `chain_parser.rs` | Befehlsketten-Parser (Operatoren `;`, `&`, `\|`) |
| `chain_exec.rs` | Kettenausführung: strikte Sequenz (`;`), permissiv (`&`), Fallback (`\|`) |
| `discovery.rs` | `help`- und `list`-Befehle: Domänen- und Aktionsentdeckung |
| `logging.rs` | Strukturiertes JSON-Logging, Maskierung sensibler Argumente |
| `output.rs` | JSON-Antwort, Exit-Codes |
| `lib.rs` | Stellt `crypto` für das Proof-Binary und Fuzz-Helpers bereit |

Jedes Modul hat seine Testdatei (`*_tests.rs`) im selben Verzeichnis.

Ein Hilfsbinary `proof` (`src/bin/proof.rs`) berechnet Authentifizierungs-Proofs für E2E-Tests und Client-Integration.

## Header-Protokoll

SSH-Frontière verwendet ein Textprotokoll über stdin/stdout. Die Präfixe unterscheiden sich je nach Richtung:

**Client zum Server (stdin):**

| Präfix | Rolle |
|--------|-------|
| `+ ` | **Konfiguriert**: Direktiven (`auth`, `session`, `body`) |
| `# ` | **Kommentiert**: vom Server ignoriert |
| *(Klartext)* | **Befehl**: `Domäne Aktion [Argumente]` |
| `.` *(allein in einer Zeile)* | **Blockende**: beendet einen Befehlsblock |

**Server zum Client (stdout):**

| Präfix | Rolle |
|--------|-------|
| `#> ` | **Kommentiert**: Banner, informative Nachrichten |
| `+> ` | **Konfiguriert**: Capabilities, Challenge-Nonce |
| `>>> ` | **Antwortet**: finale JSON-Antwort |
| `>> ` | **Stdout**: Standard-Ausgabe im Streaming (ADR 0011) |
| `>>! ` | **Stderr**: Fehlerausgabe im Streaming |

### Verbindungsablauf

```
CLIENT                                  SERVER
  |                                        |
  |  <-- Banner + Capabilities ----------  |   #> ssh-frontiere 0.1.0
  |                                        |   +> capabilities rbac, session, help, body
  |                                        |   +> challenge nonce=a1b2c3...
  |                                        |   #> type "help" for available commands
  |                                        |
  |  --- +auth (optional) ------------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (optional) ---------->   |   + session keepalive
  |                                        |
  |  --- Befehl (Klartext) ------------>   |   forgejo backup-config
  |  --- Blockende -------------------->   |   .
  |  <-- Streaming stdout -------------   |   >> Backup completed
  |  <-- Finale JSON-Antwort ----------   |   >>> {"status_code":0,"status_message":"executed",...}
  |                                        |
  |  (bei Session Keepalive)               |
  |  --- Befehl 2 --------------------->   |   infra healthcheck
  |  --- Blockende -------------------->   |   .
  |  <-- JSON-Antwort 2 --------------   |   >>> {"status_code":0,...}
  |  --- Sitzungsende (leerer Block) ->   |   .
  |  <-- Session closed ---------------   |   #> session closed
```

### JSON-Antwort

Jeder Befehl erzeugt eine finale JSON-Antwort in einer einzelnen Zeile, mit dem Präfix `>>>`. Standardausgabe und Fehler werden per Streaming über `>>` und `>>!` gesendet:

```
>> Backup completed
>>> {"command":"forgejo backup-config","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` in der finalen JSON-Antwort: die Ausgabe wurde per Streaming über `>>` und `>>!` gesendet
- Für nicht ausgeführte Befehle (Ablehnung, Config-Fehler) sind `stdout` und `stderr` ebenfalls `null`

### Body-Protokoll

Der Header `+body` ermöglicht das Übertragen mehrzeiliger Inhalte an den Kindprozess über stdin. Vier Begrenzungsmodi:

- `+body`: liest bis zu einer Zeile, die nur `.` (Punkt) enthält
- `+body size=N`: liest genau N Bytes
- `+body stop="DELIMITER"`: liest bis zu einer Zeile mit dem Delimiter
- `+body size=N stop="DELIMITER"`: erste erreichte Begrenzung (Größe oder Marker) beendet das Lesen

## TOML-Konfiguration

Das Konfigurationsformat ist deklaratives TOML. Die Wahl ist in ADR 0001 dokumentiert:

- **Warum TOML**: menschenlesbar, native Typisierung, Standard im Rust-Ökosystem, keine signifikante Einrückung (anders als YAML), ausdrucksstärker als JSON für Konfiguration.
- **Warum nicht YAML**: signifikante Einrückung fehleranfällig, gefährliche implizite Typen (`on`/`off` → Boolean), komplexe Spezifikation.
- **Warum nicht JSON**: keine Kommentare, umständlich, nicht für menschliche Konfiguration konzipiert.

Die Konfiguration wird **beim Laden validiert** (Fail-fast): TOML-Syntax, Vollständigkeit der Felder, Platzhalter-Konsistenz, mindestens eine Domäne, mindestens eine Aktion pro Domäne, nicht-leere Enum-Werte.

## Abhängigkeitspolitik

SSH-Frontière verfolgt eine Politik von **null nicht-essentiellen Abhängigkeiten**. Jede externe Crate muss durch einen echten Bedarf begründet sein.

### Aktuelle Abhängigkeiten

3 direkte Abhängigkeiten, ~20 transitive Abhängigkeiten:

| Crate | Verwendung |
|-------|------------|
| `serde` + `serde_json` | JSON-Serialisierung (Logging, Antworten) |
| `toml` | TOML-Konfiguration laden |

### Bewertungsmatrix

Vor dem Hinzufügen einer Abhängigkeit wird sie anhand von 8 gewichteten Kriterien bewertet (Note /5): Lizenz (eliminatorisch), Governance (×3), Community (×2), Aktualisierungshäufigkeit (×2), Größe (×3), transitive Abhängigkeiten (×3), Funktionalitäten (×2), Nicht-Einschluss (×1). Mindestpunktzahl: 3,5/5.

### Audit

- `cargo deny` prüft Lizenzen und bekannte Schwachstellen
- `cargo audit` durchsucht die RustSec-Datenbank nach Schwachstellen
- Autorisierte Quellen: nur crates.io

## Wie das Projekt entworfen wurde

SSH-Frontière wurde in aufeinanderfolgenden Phasen (1 bis 9, mit Zwischenphasen 2.5 und 5.5) entwickelt, gesteuert von Claude Code Agenten mit systematischer TDD-Methodik:

| Phase | Inhalt |
|-------|--------|
| 1 | Funktionaler Dispatcher, TOML-Config, 3-stufiges RBAC |
| 2 | Produktionskonfiguration, Betriebsskripte |
| 2.5 | SHA-256 FIPS 180-4, BTreeMap, Graceful Timeout |
| 3 | Einheitliches Header-Protokoll, Challenge-Response-Auth, Sitzungen |
| 4 | E2E-SSH-Docker-Tests, Code-Bereinigung, Forge-Integration |
| 5 | Sichtbarkeits-Tags, horizontale Token-Filterung |
| 5.5 | Optionaler Nonce, benannte Argumente, Proof-Binary (inkl. Phase 6, zusammengeführt) |
| 7 | Konfigurationsguide, Dry-Run `--check-config`, Help ohne Präfix |
| 8 | Strukturierte Fehlertypen, Pedantic Clippy, cargo-fuzz, Proptest |
| 9 | Body-Protokoll, freie Argumente, max_body_size, Exit-Code 133 |

Das Projekt wurde entworfen von:
- **Julien Garderon** (BO): Konzept, funktionale Spezifikationen, Rust-Wahl, Projektname
- **Claude Supervisor** (PM/Tech Lead): technische Analyse, Architektur
- **Claude Code Agenten**: Implementierung, Tests, Dokumentation

Wo Mensch und Maschine zusammenarbeiten, besser, schneller, mit mehr Sicherheit.
