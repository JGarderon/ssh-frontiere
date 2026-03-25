+++
title = "Mitwirken"
description = "Wie man zu SSH-Frontière beiträgt: Prozess, Anforderungen, Konventionen"
date = 2026-03-24
weight = 6
+++

# Zu SSH-Frontière beitragen

Beiträge sind willkommen, einschließlich durch künstliche Intelligenz unterstützte oder generierte Beiträge. SSH-Frontière wird selbst mit Claude Code Agenten entwickelt.

## Bevor Sie anfangen

Eröffnen Sie ein **Issue**, um die vorgeschlagene Änderung zu besprechen. Das vermeidet unnötige Arbeit und validiert den Ansatz.

- **Bug**: beschreiben Sie das beobachtete vs. erwartete Verhalten, Version, OS
- **Feature**: beschreiben Sie den Anwendungsfall und den geplanten Ansatz
- **Architekturänderung**: ein ADR wird benötigt (siehe `docs/decisions/`)

## Prozess

```
1. Issue       → Änderung besprechen
2. Fork        → git checkout -b feature/mein-beitrag
3. TDD         → RED (fehlschlagender Test) → GREEN (minimaler Code) → Refactoring
4. Prüfen      → make lint && make test && make audit
5. Pull Request → beschreiben, Issue referenzieren, grüne CI
```

## Qualitätsanforderungen

SSH-Frontière ist eine Sicherheitskomponente. Die Anforderungen sind streng:

| Regel | Detail |
|-------|--------|
| Testabdeckung | Mindestens 90% für hinzugefügten Code |
| Kein `unwrap()` | `expect()` mit `// INVARIANT:` oder `?` / `map_err()` verwenden |
| Kein `unsafe` | Verboten durch `#[deny(unsafe_code)]` |
| Max. 800 Zeilen | Pro Quelldatei |
| Max. 60 Zeilen | Pro Funktion |
| Formatierung | `cargo fmt` obligatorisch |
| Lints | `cargo clippy -- -D warnings` (pedantic) |

### Abhängigkeiten

**Null nicht-essentielle Abhängigkeiten.** Vor dem Vorschlag einer neuen Abhängigkeit:

1. Prüfen Sie, ob die Rust-Standardbibliothek den Bedarf nicht abdeckt
2. Bewerten Sie mit der Abhängigkeitsmatrix (Mindestpunktzahl 3,5/5)
3. Dokumentieren Sie die Bewertung in `docs/searches/`

Aktuell autorisierte Abhängigkeiten: `serde`, `serde_json`, `toml`.

## Commit-Konventionen

Nachrichten auf **Englisch**, Format `type(scope): description`:

- `feat(protocol): add TLS support`
- `fix(dispatch): handle empty arguments`
- `test(integration): add session timeout scenarios`
- `docs(references): update configuration guide`

Typen: `feat`, `fix`, `refactor`, `test`, `docs`.

## KI-Beiträge

Von KI generierte Beiträge werden unter den gleichen Bedingungen wie menschliche Beiträge akzeptiert:

- Der menschliche Beitragende **bleibt verantwortlich** für die Codequalität
- Gleiche Test- und Lint-Anforderungen
- Geben Sie im PR an, ob KI-Code verwendet wurde (Transparenz)

## Sicherheit

### Eine Schwachstelle melden

**Melden Sie Schwachstellen nicht über öffentliche Issues.** Kontaktieren Sie den Maintainer direkt für eine verantwortungsvolle Offenlegung.

### Verstärkte Prüfung

PRs, die diese Dateien betreffen, unterliegen einer verstärkten Sicherheitsprüfung:

- `protocol.rs`, `crypto.rs` — Authentifizierung
- `dispatch.rs`, `chain_parser.rs`, `chain_exec.rs` — Befehlsparsing und -ausführung
- `config.rs` — Konfigurationsverwaltung

## Gute erste Beiträge

- Dokumentation verbessern
- Tests für Grenzfälle hinzufügen
- Clippy-Warnungen beheben
- Fehlermeldungen verbessern

## Lizenz

SSH-Frontière wird unter [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12) vertrieben. Mit dem Einreichen eines Pull Requests stimmen Sie zu, dass Ihr Beitrag unter den Bedingungen dieser Lizenz verteilt wird.

Vollständige Details finden Sie in der [CONTRIBUTING.md](https://github.com/nothus-forge/ssh-frontiere/blob/main/CONTRIBUTING.md)-Datei im Repository.
