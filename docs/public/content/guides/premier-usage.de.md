+++
title = "Erste Schritte"
description = "SSH-Frontière installieren, erste Domäne konfigurieren und testen"
date = 2026-03-24
weight = 1
+++

# Erste Schritte

Diese Anleitung begleitet Sie von der Installation bis zu Ihrem ersten SSH-Befehl über SSH-Frontière.

## 1. Minimale Konfiguration vorbereiten

Erstellen Sie eine minimale `config.toml`-Datei:

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 60

[domains.test]
description = "Testdomäne"

[domains.test.actions.hello]
description = "Testbefehl, der eine Nachricht anzeigt"
level = "read"
timeout = 10
execute = "/usr/bin/echo hello from ssh-frontiere"
```

Diese Konfiguration definiert eine einzelne `test`-Domäne mit einer `hello`-Aktion, die auf der `read`-Stufe zugänglich ist.

## 2. Installieren und konfigurieren

Sie brauchen zunächst das `ssh-frontiere`-Binary. Siehe die [Kompilierungsanleitung](@/installation/compilation.md) oder laden Sie ein vorkompiliertes Binary von der [Release-Seite](https://github.com/nothus-forge/ssh-frontiere/releases) herunter.

```bash
# Binary kopieren
sudo cp ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere

# Konfiguration installieren
sudo mkdir -p /etc/ssh-frontiere
sudo cp config.toml /etc/ssh-frontiere/config.toml
sudo chmod 640 /etc/ssh-frontiere/config.toml

# Log-Verzeichnis erstellen
sudo mkdir -p /var/log/ssh-frontiere

# Dienstkonto erstellen
sudo useradd -m -s /usr/local/bin/ssh-frontiere test-user

# Dem Konto Schreibzugriff auf Logs geben
sudo chown test-user:test-user /var/log/ssh-frontiere
```

## 3. SSH-Schlüssel konfigurieren

Auf Ihrem Client-Rechner:

```bash
# Schlüssel generieren
ssh-keygen -t ed25519 -C "test-key" -f ~/.ssh/test-frontiere
```

Auf dem Server den öffentlichen Schlüssel in `~test-user/.ssh/authorized_keys` hinzufügen:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... test-key
```

```bash
# Berechtigungen sichern
sudo chmod 700 ~test-user/.ssh
sudo chmod 600 ~test-user/.ssh/authorized_keys
sudo chown -R test-user:test-user ~test-user/.ssh
```

## 4. Erster Aufruf

```bash
# Verfügbare Befehle entdecken
{ echo "help"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@server
```

Erwartete Antwort (der Server sendet zuerst das Banner, dann die Antwort):

```
#> ssh-frontiere 0.1.0
+> capabilities session, help, body
#> type "help" for available commands
#> ...
>>> {"command":"help","status_code":0,"status_message":"ok","stdout":null,"stderr":null}
```

Die `#>`-Zeilen enthalten lesbaren Hilfetext. Der `help`-Befehl zeigt die Liste der auf der `read`-Stufe zugänglichen Domänen und Aktionen an.

## 5. Einen Befehl ausführen

```bash
{ echo "test hello"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@server
```

Erwartete Antwort:

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

Die Programmausgabe (`hello from ssh-frontiere`) wird per Streaming über `>>` gesendet, dann die finale JSON-Antwort über `>>>`. Die Felder `stdout` und `stderr` sind `null` im JSON, da die Ausgabe per Streaming gesendet wurde.

## 6. Den Ablauf verstehen

Folgendes ist passiert:

1. Der SSH-Client verbindet sich mit dem Schlüssel `test-frontiere`
2. `sshd` authentifiziert den Schlüssel und liest `authorized_keys`
3. Die Option `command=` erzwingt die Ausführung von `ssh-frontiere --level=read`
4. SSH-Frontière zeigt das Banner (`#>`, `+>`) an und wartet auf Header
5. Der Client sendet den Befehl `test hello` (Klartext, ohne Präfix) dann `.` (Blockende)
6. SSH-Frontière validiert: Domäne `test`, Aktion `hello`, Stufe `read` <= erforderliche `read`
7. SSH-Frontière führt `/usr/bin/echo hello from ssh-frontiere` aus
8. Die Ausgabe wird per Streaming (`>>`) gesendet, dann die finale JSON-Antwort (`>>>`)

## 7. Eine Ablehnung testen

Versuchen Sie einen nicht existierenden Befehl:

```bash
{ echo "test nichtexistent"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@server
```

Antwort:

```
>>> {"command":"test nichtexistent","status_code":128,"status_message":"rejected: unknown action 'nichtexistent' in domain 'test'","stdout":null,"stderr":null}
```

`stdout` und `stderr` sind `null`, da der Befehl nicht ausgeführt wurde.

## Nächster Schritt

Jetzt, da SSH-Frontière funktioniert, können Sie [Ihre eigenen Domänen und Aktionen konfigurieren](@/guides/domaines.md).
