+++
title = "FAQ"
description = "Häufig gestellte Fragen zu SSH-Frontière"
date = 2026-03-24
weight = 5
+++

# Häufig gestellte Fragen

## Was ist SSH-Frontière genau?

Eine **Ersatz-Login-Shell**, geschrieben in Rust. Sie wird anstelle von `/bin/bash` in `/etc/passwd` für ein Dienstkonto installiert. Jede SSH-Verbindung durchläuft SSH-Frontière, das den Befehl gegen eine TOML-Konfigurationsdatei prüft, bevor er ausgeführt wird.

## Ist es ein SSH-Bastion?

Nein. Ein SSH-Bastion (Teleport, Boundary) ist ein **Proxy**, der Verbindungen zu anderen Servern weiterleitet. SSH-Frontière leitet nicht weiter — es kontrolliert, was **auf dem Server, auf dem es installiert ist**, ausgeführt wird.

Bastions verwalten den menschlichen Zugang zu einer Serverflotte. SSH-Frontière verwaltet den Zugang von **Dienstkonten** (CI-Runner, KI-Agenten, Skripte) zu bestimmten Aktionen auf einem Server.

## Ersetzt es `sudo`?

Nein, es ist komplementär. SSH-Frontière kontrolliert, was der SSH-Client **anfragen kann** (Schicht 2). `sudo` kontrolliert die Systemprivilegien, die **für die Ausführung nötig** sind (Schicht 3). Beide kombinieren sich für Tiefenverteidigung.

## Kann man es ohne TOML-Datei verwenden?

Nein. Die Konfigurationsdatei ist obligatorisch. Das ist beabsichtigt: alles ist explizit, deklarativ und auditierbar. Kein permissiver Modus, kein Fallback auf eine Shell.

## Was passiert bei ungültiger Konfiguration?

SSH-Frontière validiert die Konfiguration beim Start vollständig (Fail-fast). Bei ungültiger Konfiguration beendet sich das Programm mit Code 129 und einer expliziten Fehlermeldung im Log. Kein Befehl wird ausgeführt. Der SSH-Client sieht nur, dass der Dienst nicht verfügbar ist — **nie** die Fehlerdetails. Diagnoseinformationen bleiben auf der Serverseite.

Sie können die Konfiguration sicher testen:

```bash
ssh-frontiere --check-config --config /etc/ssh-frontiere/config.toml
```

## Wie diagnostiziert man ein Problem?

Mehrere Werkzeuge stehen zur Verfügung:

1. **Config-Validierung**: `ssh-frontiere --check-config` prüft Syntax und Konsistenz
2. **`help`-Befehl**: zeigt Aktionen, die auf der effektiven Ebene des Clients zugänglich sind
3. **`list`-Befehl**: Kurzversion (Domäne + Aktion)
4. **JSON-Logs**: jeder Befehl (ausgeführt oder abgelehnt) wird mit Zeitstempel, Befehl, Argumenten, Ebene, Ergebnis protokolliert
5. **Exit-Code**: 0 = Erfolg, 128 = abgelehnt, 129 = Config-Fehler, 130 = Timeout, 131 = unzureichende Stufe, 132 = Protokollfehler, 133 = Body-stdin vorzeitig geschlossen

## Können KI-Agenten es nutzen?

Ja, das ist ein erstklassiger Anwendungsfall. Die Befehle `help` und `list` geben strukturiertes JSON zurück, direkt parsbar durch einen Agenten. Das Header-Protokoll (Präfixe `+`, `#`, `$`, `>`) ist so konzipiert, dass es maschinenlesbar ist, ohne die menschliche Lesbarkeit zu beeinträchtigen.

Siehe den [KI-Agenten-Guide](@/guides/agents-ia.md) für die detaillierte Konfiguration.

## Welche Abhängigkeiten hat der Quellcode?

3 direkte Abhängigkeiten:

| Crate | Verwendung |
|-------|------------|
| `serde` + `serde_json` | JSON-Serialisierung (Logs, Antworten) |
| `toml` | Konfiguration laden |

Kein Async-Runtime, kein Tokio, kein Web-Framework. Das statische Binary ist ~1 MB groß.

## Warum Rust und nicht Go/Python?

1. **Speichersicherheit**: kein Buffer Overflow, kein Use-after-free — entscheidend für eine Sicherheitskomponente
2. **Statisches Binary**: kompiliert mit musl, keine Systemabhängigkeit
3. **Performance**: startet in Millisekunden, kein Runtime
4. **Kein `unsafe`**: verboten durch Cargo-Lints (`unsafe_code = "deny"`)

## Warum TOML und nicht YAML oder JSON?

- **TOML**: lesbar, typisiert, Kommentare, Rust-Standard, keine signifikante Einrückung
- **YAML**: signifikante Einrückung fehleranfällig, gefährliche implizite Typen (`on`/`off` → Boolean)
- **JSON**: keine Kommentare, umständlich, nicht für menschliche Konfiguration konzipiert

Die Wahl ist in ADR 0001 dokumentiert.

## Wie funktioniert die Token-Authentifizierung?

Zwei Modi:

1. **Einfacher Modus** (`challenge_nonce = false`): der Client berechnet `SHA-256(secret)` und sendet es als Proof
2. **Nonce-Modus** (`challenge_nonce = true`): der Server sendet einen Nonce, der Client berechnet `SHA-256(XOR_encrypt(secret || nonce, secret))`

Der Nonce-Modus schützt vor Replay: jeder Proof ist dank des Nonce einzigartig.

## Kann man mehrere SSH-Schlüssel verwenden?

Ja. Jeder Schlüssel in `authorized_keys` hat seine eigene `--level`. Mehrere Schlüssel können mit unterschiedlichen Stufen koexistieren:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin
```

## Wie ist das Antwortformat?

Standardausgabe und Fehler werden per Streaming gesendet (Präfixe `>>` und `>>!`), dann eine finale JSON-Antwort in einer einzelnen Zeile (Präfix `>>>`):

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` im finalen JSON: die Ausgabe wurde per Streaming gesendet
- `status_code` = 0: Erfolg (Exit-Code des Kindprozesses im Passthrough)

## Wie aktualisiert man SSH-Frontière?

1. Neue Version kompilieren (`make release`)
2. Binary auf den Server kopieren (`scp`)
3. Verifizieren (`ssh user@server` + `help`)

Keine Datenmigration, kein Datenbankschema. Die TOML-Datei ist mit Git versionierbar.

## Wie kann man beitragen?

Siehe den [Beitragsguide](@/contribuer.md). Zusammengefasst: Issue eröffnen, Fork, TDD, Pull Request, grüne CI. KI-generierte Beiträge werden akzeptiert.

## Wo findet man den Quellcode?

Der Quellcode ist im [GitHub-Repository](https://github.com/nothus-forge/ssh-frontiere) verfügbar. Lizenz [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).
