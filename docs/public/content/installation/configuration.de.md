+++
title = "Konfiguration"
description = "Die config.toml-Datei von SSH-Frontière schreiben"
date = 2026-03-24
weight = 3
+++

# Konfiguration

SSH-Frontière verwendet eine TOML-Datei zur Deklaration von Domänen, Aktionen, Zugriffsebenen, Argumenten und Authentifizierungstoken.

## Speicherort

**Standardpfad**: `/etc/ssh-frontiere/config.toml`

**Überschreibung** (nach Priorität):
1. `--config <path>` in der `command=`-Zeile von `authorized_keys`
2. Umgebungsvariable `SSH_FRONTIERE_CONFIG`
3. Standardpfad

**Empfohlene Berechtigungen**: `root:forge-runner 640` (Gruppe an das verwendete Dienstkonto anpassen).

## Dateistruktur

```toml
[global]                              # Allgemeine Einstellungen
[domains.<id>]                        # Funktionale Domänen
  [domains.<id>.actions.<id>]         # Autorisierte Aktionen
    [domains.<id>.actions.<id>.args]  # Benannte Argumente (optional)
[auth]                                # RBAC-Authentifizierung (optional)
  [auth.tokens.<id>]                  # Token mit Secret, Stufe und Tags
```

## Abschnitt `[global]`

| Schlüssel | Typ | Standard | Beschreibung |
|-----------|-----|----------|--------------|
| `log_file` | String | **erforderlich** | Pfad zur JSON-Logdatei |
| `default_timeout` | Integer | `300` | Standard-Timeout in Sekunden |
| `max_stdout_chars` | Integer | `65536` | Stdout-Limit (64 KB) |
| `max_stderr_chars` | Integer | `16384` | Stderr-Limit (16 KB) |
| `max_output_chars` | Integer | `131072` | Globales Hard-Limit (128 KB) |
| `max_stream_bytes` | Integer | `10485760` | Streaming-Volumenlimit (10 MB) |
| `timeout_session` | Integer | `3600` | Session-Keepalive-Timeout |
| `max_auth_failures` | Integer | `3` | Auth-Versuche vor Lockout |
| `ban_command` | String | `""` | IP-Ban-Befehl (Platzhalter `{ip}`) |
| `log_comments` | Bool | `false` | Client-`#`-Zeilen protokollieren |
| `expose_session_id` | Bool | `false` | Sitzungs-UUID im Banner anzeigen |

Die Schlüssel `log_level`, `default_level` und `mask_sensitive` werden vom Parser für Abwärtskompatibilität akzeptiert, aber nicht mehr verwendet.

## Abschnitt `[domains]`

Eine **Domäne** ist ein funktionaler Bereich (z.B. `forgejo`, `infra`, `notify`). Jede Domäne enthält autorisierte **Aktionen**.

```toml
[domains.forgejo]
description = "Git-Forge-Infrastruktur"

[domains.forgejo.actions.backup-config]
description = "Konfiguration sichern"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
max_body_size = 65536       # Body-Limit (64 KB, optional)
```

Jede Aktion akzeptiert folgende Schlüssel: `description` (erforderlich), `level` (erforderlich), `execute` (erforderlich), `timeout` (optional, überschreibt global), `tags` (optional), `max_body_size` (optional, Standard 65536 Bytes).

### Vertrauensstufen

Strikte Hierarchie: `read` < `ops` < `admin`

| Stufe | Verwendung |
|-------|------------|
| `read` | Nur lesen: healthcheck, status, list |
| `ops` | Routineoperationen: backup, deploy, restart |
| `admin` | Alle Aktionen + Administration |

### Argumente

Argumente werden als TOML-Dictionary deklariert:

```toml
[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }
```

| Feld | Typ | Beschreibung |
|------|-----|--------------|
| `type` | String | `"enum"` oder `"string"` |
| `values` | Liste | Erlaubte Werte (für `enum`) |
| `default` | String | Standardwert (macht das Argument optional) |
| `sensitive` | Bool | Wenn `true`, in Logs maskiert |
| `free` | Bool | Wenn `true`, akzeptiert jeden Wert ohne Einschränkung |

### Platzhalter in `execute`

| Platzhalter | Beschreibung |
|-------------|--------------|
| `{domain}` | Domänenname (immer verfügbar) |
| `{arg_name}` | Wert des entsprechenden Arguments |

### Sichtbarkeits-Tags

Tags filtern den Zugang zu Aktionen horizontal. Eine Aktion ohne Tags ist für alle zugänglich. Eine Aktion mit Tags ist nur für Identitäten zugänglich, deren Tags mindestens einen gemeinsamen Tag haben.

```toml
[domains.forgejo.actions.deploy]
# ...
tags = ["forgejo", "deploy"]
```

## Abschnitt `[auth]` (optional)

RBAC-Authentifizierung ermöglicht Rechteeskalation über Challenge-Response:

```toml
[auth]
challenge_nonce = false              # true = Anti-Replay-Nonce-Modus

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Base64-kodiertes Secret
level = "ops"                               # Gewährte Stufe
tags = ["forgejo"]                          # Sichtbarkeits-Tags
```

Das Secret muss mit `b64:` prefixed und base64-kodiert sein. Um ein Secret zu generieren:

```bash
echo -n "mein-zufaelliges-secret" | base64
# bWVpbi16dWZhZWxsaWdlcy1zZWNyZXQ=
```

## Validierung beim Laden

Die Konfiguration wird bei jedem Laden vollständig validiert (Fail-fast). Bei Fehler beendet sich das Programm mit Code 129. Validierungen:

- Korrekte TOML-Syntax
- Mindestens eine Domäne, mindestens eine Aktion pro Domäne
- Jede Aktion hat ein gültiges `execute` und `level`
- Platzhalter `{arg}` in `execute` stimmen mit deklarierten Argumenten überein
- Enum-Argumente haben mindestens einen erlaubten Wert
- Standardwerte sind in der Liste erlaubter Werte
- `max_stdout_chars` und `max_stderr_chars` <= `max_output_chars`

## Vollständiges Beispiel

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300
max_stdout_chars = 65536
max_stderr_chars = 16384
max_output_chars = 131072
timeout_session = 3600
max_auth_failures = 3

[domains.forgejo]
description = "Git-Forge-Infrastruktur"

[domains.forgejo.actions.backup-config]
description = "Forgejo-Konfiguration sichern"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"

[domains.forgejo.actions.deploy]
description = "Deployment mit Versions-Tag"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.infra]
description = "Server-Infrastruktur"

[domains.infra.actions.healthcheck]
description = "Dienst-Gesundheitsprüfung"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[auth]
challenge_nonce = false

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

Für einen detaillierten Guide mit allen Anwendungsfällen, siehe den [vollständigen Konfigurationsguide](https://github.com/nothus-forge/ssh-frontiere/blob/main/docs/references/configuration.md) im Repository.

---

**Weiter**: [Deployment](@/installation/deploiement.md) — in Produktion nehmen.
