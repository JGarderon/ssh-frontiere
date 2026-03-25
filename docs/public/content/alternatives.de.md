+++
title = "Alternativen"
description = "Vergleich von SSH-Frontière mit bestehenden SSH-Kontrolllösungen"
date = 2026-03-24
weight = 4
+++

# Vergleich mit Alternativen

SSH-Frontière ist nicht die einzige Möglichkeit, SSH-Zugriffe zu kontrollieren. Diese Seite vergleicht bestehende Ansätze, um Ihnen bei der Wahl der richtigen Lösung zu helfen.

## Vergleichstabelle

| Kriterium | `authorized_keys` `command=` | SSH-Frontière | Teleport | Boundary |
|-----------|------------------------------|---------------|----------|----------|
| **Typ** | OpenSSH-Option | Login-Shell | SSH-Bastion | SSH-Bastion |
| **Zielgruppe** | Ein Skript pro Schlüssel | Dienstkonten | Menschliche Benutzer | Menschliche Benutzer |
| **Granularität** | 1 Befehl pro Schlüssel | 3-stufiges RBAC, Domänen, Aktionen, Argumente | Rollen, Labels, RBAC | IAM-Richtlinien |
| **Logging** | Unstrukturiert | Strukturiertes JSON pro Befehl | Vollständige Sitzung (Replay) | Audit-Trail |
| **Deployment** | Nativ (OpenSSH) | 1 Binary + 1 TOML-Datei | Cluster (Auth-Server, Proxy, Node) | Cluster (Controller, Workers) |
| **Abhängigkeiten** | Keine | 0 Systemabhängigkeiten | Datenbank, Zertifikate | Datenbank |
| **Größe** | — | ~1 MB (statisches Binary) | ~100 MB | ~100 MB |
| **Anti-Injection** | Verantwortung des Skripts | Strukturell (grammatischer Parser) | N/A (interaktive Sitzung) | N/A (interaktive Sitzung) |
| **LLM-kompatibel** | Nein | Ja (JSON, Help, Discovery) | Nein | Nein |
| **Lizenz** | OpenSSH (BSD) | EUPL-1.2 | AGPL-3.0 (OSS) / Kommerziell | BSL 1.1 |

## `authorized_keys` mit `command=`

Die Option `command=` in `authorized_keys` erzwingt die Ausführung eines Skripts bei jeder Verbindung. Es ist die einfachste und am weitesten verbreitete Lösung.

### Vorteile

- **Keine Installation**: native OpenSSH-Funktion
- **Einfach** für einen einzelnen Anwendungsfall (ein Schlüssel = ein Befehl)

### Einschränkungen

- **Ein Skript pro Schlüssel**: keine feine Granularität. Für N verschiedene Aktionen braucht man N Schlüssel oder ein Bash-Skript, das `$SSH_ORIGINAL_COMMAND` parst
- **Keine Argumentvalidierung**: das Skript empfängt eine Rohzeichenkette und muss sie selbst validieren — Injection-Quelle bei schlechter Umsetzung
- **Keine Zugriffsebenen**: alle Schlüssel haben die gleichen Rechte (oder man muss sie ins Skript kodieren)
- **Kein strukturiertes Logging**: Logs hängen vom Skript ab
- **Fragil**: ein Bash-Skript mit Befehlsvalidierung ist schwer abzusichern und zu warten

### Wann `command=` wählen

- Einfacher Bedarf: ein SSH-Schlüssel, ein fester Befehl, keine Parameter
- Keine Audit- oder RBAC-Anforderung

## Teleport

[Teleport](https://goteleport.com/) ist ein vollständiges SSH-Bastion mit Sitzungsaufzeichnung, SSO, Zertifikaten und Audit-Trail.

### Vorteile

- **Sitzungsaufzeichnung**: vollständiges Replay jeder SSH-Sitzung
- **Integriertes SSO**: GitHub, OIDC, SAML
- **Zertifikate**: keine SSH-Schlüsselverwaltung
- **Vollständiges Audit**: wer sich verbunden hat, wann, woher, was gemacht wurde

### Einschränkungen

- **Komplex zu deployen**: Auth-Server, Proxy, Node-Agent, Datenbank, Zertifikate
- **Für Menschen konzipiert**: interaktive Sitzungen, kein Machine-to-Machine-Protokoll
- **Überdimensioniert** für Dienstkonten: ein CI-Runner braucht keine Sitzungsaufzeichnung und kein SSO
- **Duale Lizenz**: die Community-Version (AGPL-3.0) hat funktionale Einschränkungen

### Wann Teleport wählen

- Verwaltung des **menschlichen** Zugangs zu einer Serverflotte
- Bedarf an Sitzungsaufzeichnung und SSO
- Infrastruktur mit Ressourcen für Deployment und Wartung eines Clusters

## HashiCorp Boundary

[Boundary](https://www.boundaryproject.io/) ist ein Zugangs-Proxy, der Verbindungsdetails abstrahiert und externe Identitätsquellen integriert.

### Vorteile

- **Infrastrukturabstraktion**: Benutzer verbinden sich mit logischen Zielen, nicht mit IPs
- **IAM-Integration**: Active Directory, OIDC, LDAP
- **Credential-Injection**: Secrets werden dynamisch injiziert, nie geteilt

### Einschränkungen

- **Komplex**: Controller, Workers, Datenbank, IAM-Integration
- **Auf Menschen ausgerichtet**: nicht für automatisierte Skripte konzipiert
- **BSL 1.1 Lizenz**: kommerzielle Einschränkungen bei der Community-Edition
- **Keine Kontrolle auf Befehlsebene**: Boundary kontrolliert den Zugang zu einem Host, nicht zu einem bestimmten Befehl

### Wann Boundary wählen

- Große Serverflotte mit zentralisierter Identitätsverwaltung
- Bedarf an Infrastrukturabstraktion (Benutzer kennen die IPs nicht)
- Team mit HashiCorp-Expertise (Vault, Terraform usw.)

## `sudo` allein

`sudo` kontrolliert die Rechteeskalation für Systembefehle. Wird oft allein verwendet, um Dienstkonto-Aktionen einzuschränken.

### Vorteile

- **Nativ**: auf allen Linux-Systemen vorhanden
- **Granular**: feine Regeln pro Benutzer, Befehl und Argumente

### Einschränkungen

- **Kontrolliert nicht die SSH-Eingabe**: jeder Befehl kann via SSH **angefragt** werden, selbst wenn `sudo` die Eskalation blockiert
- **Kein Protokoll**: keine strukturierte Antwort, kein integriertes JSON-Logging
- **Komplexe Konfiguration**: sudoers-Regeln werden bei vielen Befehlen schwer wartbar

### Wann `sudo` allein wählen

- Einfache Umgebung mit geringem Risiko
- SSH-Eingabe wird bereits durch einen anderen Mechanismus kontrolliert (Bastion, VPN)

## Wann SSH-Frontière wählen

SSH-Frontière ist für einen **spezifischen Anwendungsfall** konzipiert: kontrollieren, was Dienstkonten (nicht Menschen) via SSH tun können.

Wählen Sie SSH-Frontière, wenn:

- Ihre SSH-Verbindungen **automatisierte Skripte** sind (CI/CD, KI-Agenten, Cron)
- Sie **Granularität** brauchen: Domänen, Aktionen, Argumente, Zugriffsebenen
- Sie **strukturiertes JSON-Logging** für Audit und Observability wollen
- Sie ein **einfaches Deployment** wollen: ein Binary, eine TOML-Datei
- Sie **LLM-Kompatibilität** brauchen: JSON-Antworten, Discovery über `help`/`list`
- Sie keinen Cluster deployen und warten wollen (Teleport, Boundary)

Wählen Sie SSH-Frontière nicht, wenn:

- Ihre Benutzer **Menschen** sind, die reichhaltige und vollständige interaktive Sitzungen brauchen
- Sie einen **SSH-Proxy** zu anderen Servern brauchen
- Sie **SSO** brauchen
