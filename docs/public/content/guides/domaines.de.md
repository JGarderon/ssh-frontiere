+++
title = "Domänen und Aktionen"
description = "Domänen und Aktionen in SSH-Frontière konfigurieren"
date = 2026-03-24
weight = 2
+++

# Domänen und Aktionen konfigurieren

Eine **Domäne** ist ein funktionaler Bereich (eine Anwendung, ein Dienst, eine Kategorie von Operationen). Jede Domäne enthält **Aktionen**: die autorisierten Befehle.

## Eine Deployment-Domäne hinzufügen

```toml
[domains.meineapp]
description = "Haupt-Webanwendung"

[domains.meineapp.actions.deploy]
description = "Eine Version deployen"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy-meineapp.sh {tag}"

[domains.meineapp.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.meineapp.actions.status]
description = "Dienststatus prüfen"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-meineapp.sh"

[domains.meineapp.actions.restart]
description = "Dienst neustarten"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-meineapp.sh"
```

Verwendung:

```bash
# Stabile Version deployen
{ echo "meineapp deploy version=stable"; echo "."; } | ssh ops@server

# Status prüfen
{ echo "meineapp status"; echo "."; } | ssh monitoring@server

# Neustarten
{ echo "meineapp restart"; echo "."; } | ssh ops@server
```

## Eine Backup-Domäne hinzufügen

```toml
[domains.backup]
description = "Automatisierte Sicherungen"

[domains.backup.actions.full]
description = "Vollständige Sicherung"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-full.sh {domain}"

[domains.backup.actions.config-only]
description = "Nur Konfiguration sichern"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
```

## Eine Benachrichtigungs-Domäne hinzufügen

```toml
[domains.notify]
description = "Benachrichtigungen"

[domains.notify.actions.slack]
description = "Slack-Benachrichtigung senden"
level = "ops"
timeout = 30
execute = "/usr/local/bin/notify-slack.sh {channel} {message}"

[domains.notify.actions.slack.args]
channel = { type = "enum", values = ["general", "ops", "alerts"], default = "ops" }
message = { free = true }
```

Das Argument `message` ist mit `free = true` deklariert: es akzeptiert jeden Textwert.

```bash
{ echo 'notify slack channel=ops message="Deployment abgeschlossen"'; echo "."; } | ssh ops@server
```

## Eine Wartungs-Domäne hinzufügen

```toml
[domains.infra]
description = "Server-Infrastruktur"

[domains.infra.actions.healthcheck]
description = "Dienst-Gesundheitsprüfung"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[domains.infra.actions.disk-usage]
description = "Speicherplatz"
level = "read"
timeout = 10
execute = "/usr/bin/df -h"

[domains.infra.actions.logs]
description = "Aktuelle Systemlogs"
level = "ops"
timeout = 30
execute = "sudo /usr/bin/journalctl -n 100 --no-pager"
```

## Checkliste nach dem Hinzufügen einer Aktion

1. TOML-Syntax prüfen (Fehler = Fail-fast, Code 129)
2. Ausführungsskript erstellen falls nötig
3. In sudoers hinzufügen falls der Befehl `sudo` verwendet
4. Mit `ssh user@server` von einem anderen Terminal testen
5. Logs in `/var/log/ssh-frontiere/commands.json` prüfen

## Entdeckung

Die Befehle `help` und `list` zeigen die verfügbaren Aktionen:

```bash
# Vollständige Liste mit Beschreibungen (lesbarer Text über #>)
{ echo "help"; echo "."; } | ssh user@server

# Domänendetails (lesbarer Text über #>)
{ echo "help meineapp"; echo "."; } | ssh user@server

# Kurzliste als JSON (Domäne + Aktion)
{ echo "list"; echo "."; } | ssh user@server
```

`help` gibt lesbaren Text zurück (Präfix `#>`). `list` gibt strukturiertes JSON zurück — besser geeignet für automatisches Parsing. Beide zeigen nur Aktionen, die auf der effektiven Stufe des Clients zugänglich sind.

---

**Weiter**: [Token und Sicherheitsstufen](@/guides/tokens.md) — kontrollieren, wer was tun darf.
