+++
title = "Vorstellung"
description = "SSH-Frontière entdecken: was es ist, warum es existiert, wie es funktioniert"
date = 2026-03-24
weight = 1
+++

# Vorstellung von SSH-Frontière

## Das Problem

Auf einem Linux-Server verwenden SSH-Dienstkonten (CI-Runner, KI-Agenten, Wartungsskripte) in der Regel `/bin/bash` als Login-Shell. Das verursacht mehrere Probleme:

- **Keine Kontrolle**: der SSH-Client kann jeden beliebigen Befehl ausführen
- **Kein Audit**: ausgeführte Befehle werden nicht strukturiert protokolliert
- **Keine Granularität**: ein Skript, das nur einen Status lesen muss, hat die gleichen Rechte wie ein Deployment-Skript

Die klassischen Lösungen (`authorized_keys` mit `command=`, Bash-Wrapper-Skripte, SSH-Bastions) haben jeweils ihre Grenzen: fragil, schwer zu auditieren oder überdimensioniert für den Bedarf.

## Was SSH-Frontière macht

SSH-Frontière ist eine **Ersatz-Login-Shell**. Sie befindet sich zwischen `sshd` und den Systembefehlen:

```
SSH-Client
    |
    v
sshd (Schlüssel-Authentifizierung)
    |
    v
ssh-frontiere (Login-Shell)
    |
    ├── Validiert den Befehl gegen die TOML-Konfiguration
    ├── Prüft die Zugriffsebene (read / ops / admin)
    ├── Führt den autorisierten Befehl aus
    └── Gibt das Ergebnis als strukturiertes JSON zurück
```

Jede SSH-Verbindung erzeugt einen neuen `ssh-frontiere`-Prozess, der:

1. Ein Banner und die Server-Capabilities anzeigt
2. Die Client-Header liest (Authentifizierung, Sitzungsmodus)
3. Den Befehl liest (`Domäne Aktion [Argumente]`, Klartext)
4. Gegen die TOML-Whitelist validiert
5. Bei Genehmigung ausführt, andernfalls ablehnt
6. Eine JSON-Antwort zurückgibt und sich beendet

Das Programm ist **synchron und kurzlebig**: kein Daemon, kein Dienst, kein persistenter Zustand.

## Was SSH-Frontière nicht macht

- **Kein SSH-Bastion**: kein Proxy, keine Verbindungsweiterleitung zu anderen Servern
- **Kein Schlüsselmanager**: die SSH-Schlüsselverwaltung bleibt in `authorized_keys` und `sshd`
- **Keine Shell**: keine Befehlsinterpretation, keine Pipes, keine Umleitung, keine Interaktivität
- **Kein Daemon**: wird bei jeder Verbindung gestartet und beendet

## Konkrete Anwendungsfälle

### CI/CD-Automatisierung

Ein Forgejo Actions Runner deployt eine Anwendung via SSH:

```bash
# Der Runner sendet den Befehl via SSH
{
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@server
```

SSH-Frontière prüft, dass der Runner die Ebene `admin` hat, dass die Aktion `deploy` in der Domäne `forgejo` existiert, dass das Argument `version=stable` ein erlaubter Wert ist, und führt dann das konfigurierte Deployment-Skript aus.

### KI-Agenten

Ein Claude Code Agent operiert auf einem Server mit begrenzten Rechten:

```bash
# Der Agent entdeckt verfügbare Befehle
{ echo "list"; echo "."; } | ssh agent-ia@server

# Der Agent führt eine bestimmte Aktion aus
{ echo "infra healthcheck"; echo "."; } | ssh agent-ia@server
```

Der Agent hat nur Zugriff auf die für ihn konfigurierten `read`-Aktionen. Die Befehle `help` und `list` ermöglichen es ihm, verfügbare Aktionen und ihre Parameter zu entdecken — JSON-Format, nativ parsbar.

### Automatisierte Wartung

Cron-Skripte führen Backups via SSH durch:

```bash
# Nächtliches Backup
{ echo "forgejo backup-config"; echo "."; } | ssh backup@server

# Benachrichtigung nach Deployment
{ echo 'notify send message="Deployment abgeschlossen"'; echo "."; } | ssh notify@server
```

### Benachrichtigungen

Benachrichtigungen (Slack, Olvid, E-Mail) als Standard-SSH-Frontière-Aktionen auslösen:

```bash
{ echo 'notify slack channel=ops message="Build OK"'; echo "."; } | ssh notify@server
```

## Warum SSH-Frontière statt...

### ...Bash-Skripte in `authorized_keys`?

Die Option `command=` in `authorized_keys` erlaubt das Erzwingen eines Befehls, aber:
- Ein Skript pro Schlüssel — keine Granularität
- Keine Argumentvalidierung
- Keine Zugriffsebenen
- Kein strukturiertes Logging
- Das Bash-Skript kann Schwachstellen enthalten (Injection, Globbing)

SSH-Frontière bietet deklarative Konfiguration, RBAC, JSON-Logging und einen grammatischen Parser, der Injections eliminiert.

### ...ein SSH-Bastion (Teleport, Boundary)?

SSH-Bastions sind für die Verwaltung des **menschlichen** Zugangs zu Servern konzipiert:
- Aufwändig zu deployen und zu warten
- Überdimensioniert für Dienstkonten
- Anderes Bedrohungsmodell (interaktiver Benutzer vs. automatisiertes Skript)

SSH-Frontière ist eine leichtgewichtige Komponente (~1 MB), die für **Dienstkonten** konzipiert ist: keine interaktive Sitzung, kein Proxy, nur Befehlsvalidierung.

### ...`sudo` allein?

`sudo` kontrolliert die Rechteeskalation, aber:
- Kontrolliert nicht, was der SSH-Client *anfragen* kann
- Kein strukturiertes Protokoll (JSON-Ein-/Ausgabe)
- Kein integriertes Logging auf SSH-Befehlsebene

SSH-Frontière und `sudo` sind komplementär: SSH-Frontière validiert den eingehenden Befehl, `sudo` kontrolliert die Systemprivilegien. Das sind Schicht 2 und Schicht 3 der Tiefenverteidigung.

## Der Produktwert

SSH-Frontière bringt **deklarative Governance** für SSH-Dienstzugriffe:

1. **Alles in einer TOML-Datei**: Domänen, Aktionen, Argumente, Zugriffsebenen. Keine über Skripte verstreute Logik.

2. **Sofortiges Deployment**: Da die gesamte Konfiguration in einer einzigen TOML-Datei zentralisiert ist, ist das Deployen einer neuen Version trivial. Jede SSH-Verbindung erzeugt einen neuen Prozess, der die Konfiguration neu einliest — Änderungen werden am Ende der aktuellen Sitzung oder sofort für jeden neuen Client wirksam.

3. **Null Vertrauen standardmäßig**: Nichts wird ausgeführt, ohne explizit konfiguriert zu sein. Keine Shell, keine Injection möglich.

4. **Auditierbar**: Jeder Versuch (autorisiert oder abgelehnt) wird in strukturiertem JSON protokolliert mit Zeitstempel, Befehl, Argumenten, Ebene, Ergebnis.

5. **LLM-kompatibel**: KI-Agenten können verfügbare Aktionen über `help`/`list` entdecken und über ein strukturiertes JSON-Protokoll interagieren — kein Parsen von Freitext nötig.

6. **Europäisch und Open Source**: EUPL-1.2-Lizenz, in Frankreich entwickelt, keine Abhängigkeit von einem proprietären Ökosystem.

---

Weiterführend: [Installation](@/installation/_index.md) | [Architektur](@/architecture.md) | [Sicherheit](@/securite.md) | [Alternativen](@/alternatives.md)
