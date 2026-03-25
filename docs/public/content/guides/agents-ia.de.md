+++
title = "KI-Agenten"
description = "SSH-Frontière mit KI-Agenten nutzen (Claude Code usw.)"
date = 2026-03-24
weight = 4
+++

# SSH-Frontière mit KI-Agenten nutzen

SSH-Frontière wurde von Anfang an für die Kompatibilität mit KI-Agenten (LLMs) konzipiert. Das strukturierte Protokoll, die automatische Entdeckung und die JSON-Antworten machen es zu einem idealen Einstiegspunkt für Agenten, die auf einem Server handeln müssen.

## Warum SSH-Frontière für KI-Agenten?

KI-Agenten (Claude Code, Cursor, GPT usw.) können Befehle auf einem Server via SSH ausführen. Das Problem: ohne Kontrolle kann ein Agent alles ausführen.

SSH-Frontière löst dieses Problem:

- **Aktionen begrenzen**: der Agent kann nur konfigurierte Befehle ausführen
- **Zugriffsebenen**: ein Agent auf `read` kann nur konsultieren, nicht ändern
- **Entdeckung**: der Agent kann `help` fragen, um verfügbare Aktionen zu erfahren
- **Strukturiertes JSON**: Antworten sind direkt vom Agenten parsbar

## Konfiguration für einen KI-Agenten

### 1. Dedizierter SSH-Schlüssel

Generieren Sie einen SSH-Schlüssel für den Agenten:

```bash
ssh-keygen -t ed25519 -C "agent-claude" -f ~/.ssh/agent-claude
```

### 2. Eingeschränkte Vertrauensstufe

In `authorized_keys` eine minimale Stufe vergeben:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... agent-claude
```

Mit `read` beginnen und bei Bedarf über ein Token erhöhen.

### 3. Dedizierte Domänen

Spezifische Aktionen für den Agenten konfigurieren:

```toml
[domains.agent]
description = "Aktionen für KI-Agenten"

[domains.agent.actions.status]
description = "Dienststatus"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-all.sh"

[domains.agent.actions.logs]
description = "Aktuelle Anwendungslogs"
level = "read"
timeout = 30
execute = "/usr/local/bin/recent-logs.sh {service}"

[domains.agent.actions.logs.args]
service = { type = "enum", values = ["web", "api", "worker", "database"] }

[domains.agent.actions.restart]
description = "Einen Dienst neustarten"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-service.sh {service}"
tags = ["agent-ops"]

[domains.agent.actions.restart.args]
service = { type = "enum", values = ["web", "api", "worker"] }
```

### 4. Token für Erhöhung (optional)

Wenn der Agent Zugang zu `ops`-Aktionen braucht:

```toml
[auth.tokens.agent-claude]
secret = "b64:c2VjcmV0LWFnZW50LWNsYXVkZQ=="
level = "ops"
tags = ["agent-ops"]
```

## Beispiel mit Claude Code (AutoClaude)

Ein Claude Code Agent in einem AutoClaude-Container kann SSH-Frontière nutzen, um auf dem Host-Server zu agieren:

```bash
# Der Agent entdeckt verfügbare Befehle (JSON über list)
{ echo "list"; echo "."; } | ssh -i /keys/agent-claude agent@server

# Der Agent prüft den Dienststatus
{ echo "agent status"; echo "."; } | ssh -i /keys/agent-claude agent@server

# Der Agent liest Dienst-Logs
{ echo "agent logs service=api"; echo "."; } | ssh -i /keys/agent-claude agent@server
```

Die Ausgabe wird per Streaming (`>>`) gesendet, dann die finale JSON-Antwort (`>>>`):

```
>> web: running
>> api: running
>> worker: stopped
>>> {"command":"agent status","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

Der Agent kann die `>>`-Zeilen (Standardausgabe per Streaming) analysieren, erkennen dass `worker` gestoppt ist, und entsprechend handeln. Die `>>>`-Antwort bestätigt den Rückgabecode.

## Sitzungsmodus

Um nicht für jeden Befehl eine SSH-Verbindung zu öffnen, kann der Agent den Sitzungsmodus nutzen:

```bash
{
  echo "+ auth token=agent-claude proof=..."
  echo "+ session keepalive"
  echo "agent status"
  echo "."
  echo "agent logs service=worker"
  echo "."
  echo "."   # leerer Block = Sitzungsende
} | ssh -i /keys/agent-claude agent@server
```

Jeder Befehl wird von `.` (Blockende) gefolgt. Ein `.` ohne vorherigen Befehl signalisiert das Sitzungsende. Der Sitzungsmodus ermöglicht das Senden mehrerer Befehle in einer einzigen SSH-Verbindung, mit einem konfigurierbaren Gesamt-Timeout (`timeout_session`).

## Best Practices

1. **Prinzip der minimalen Rechte**: mit `read` beginnen, nur bei Bedarf per Token erhöhen
2. **Atomare Aktionen**: jede Aktion macht eine Sache. Der Agent komponiert Aktionen untereinander
3. **Explizite Namen**: Domänen- und Aktionsnamen sind über `help` sichtbar — machen Sie sie verständlich
4. **Sichtbarkeits-Tags**: Agenten-Aktionen mit dedizierten Tags isolieren
5. **Ausgabelimits**: `max_stdout_chars` konfigurieren, um zu verhindern dass der Agent zu große Datenmengen empfängt
6. **Logs**: Logs überwachen, um abnormale Nutzung zu erkennen

---

**Weiter**: [CI/CD-Integration](@/guides/ci-cd.md) — Deployments über SSH-Frontière automatisieren.
