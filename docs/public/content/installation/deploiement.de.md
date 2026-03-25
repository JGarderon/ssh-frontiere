+++
title = "Deployment"
description = "SSH-Frontière auf einem Server in Produktion nehmen"
date = 2026-03-24
weight = 4
+++

# Deployment

Das Deployment von SSH-Frontière erfolgt in 4 Schritten: Binary installieren, SSH-Schlüssel konfigurieren, Login-Shell ändern und mit sudoers absichern.

## 1. Binary installieren

```bash
# Binary auf den Server kopieren
scp target/x86_64-unknown-linux-musl/release/ssh-frontiere root@server:/usr/local/bin/

# Auf dem Server
chmod 755 /usr/local/bin/ssh-frontiere
```

## 2. Konfiguration installieren

```bash
# Verzeichnis erstellen
mkdir -p /etc/ssh-frontiere

# Konfiguration kopieren
cp config.toml /etc/ssh-frontiere/config.toml

# Berechtigungen sichern (das Dienstkonto muss die Config lesen können)
chown root:forge-runner /etc/ssh-frontiere/config.toml
chmod 640 /etc/ssh-frontiere/config.toml

# Log-Verzeichnis erstellen
mkdir -p /var/log/ssh-frontiere
chown forge-runner:forge-runner /var/log/ssh-frontiere
chmod 755 /var/log/ssh-frontiere
```

## 3. Dienstkonto erstellen

```bash
# Benutzer mit ssh-frontiere als Login-Shell erstellen
useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

Oder, wenn das Konto bereits existiert:

```bash
# Login-Shell ändern
chsh -s /usr/local/bin/ssh-frontiere forge-runner
```

**Vorsicht**: Schließen Sie Ihre aktuelle Sitzung nicht, bis Sie überprüft haben, dass die SSH-Verbindung von einer anderen Sitzung aus funktioniert.

## 4. SSH-Schlüssel konfigurieren (Schicht 1)

Bearbeiten Sie `~forge-runner/.ssh/authorized_keys`:

```
# CI-Runner-Schlüssel (ops-Stufe)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci

# Monitoring-Schlüssel (nur read-Stufe)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring

# Admin-Schlüssel (admin-Stufe)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-deploy
```

Die Option `command=` erzwingt die Ausführung von `ssh-frontiere` mit der gewählten `--level`, unabhängig vom Befehl des Clients. Die Option `restrict` deaktiviert Port-Forwarding, Agent-Forwarding, PTY und X11.

```bash
# Berechtigungen sichern
chmod 700 ~forge-runner/.ssh
chmod 600 ~forge-runner/.ssh/authorized_keys
chown -R forge-runner:forge-runner ~forge-runner/.ssh
```

## 5. Sudoers konfigurieren (Schicht 3)

Erstellen Sie `/etc/sudoers.d/ssh-frontiere`:

```
# SSH-Frontière: autorisierte Befehle für das Dienstkonto
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/healthcheck.sh
```

Der Wildcard `*` ist nötig für Skripte, die Argumente erhalten (z.B. `backup-config.sh forgejo`). Skripte ohne Argumente (wie `healthcheck.sh`) brauchen ihn nicht.

Syntax validieren:

```bash
visudo -c -f /etc/sudoers.d/ssh-frontiere
```

## 6. Überprüfen

```bash
# Von einem anderen Terminal testen (aktuelle Sitzung nicht schließen)

# Prüfen, dass verfügbare Befehle angezeigt werden
{ echo "help"; echo "."; } | ssh forge-runner@server

# Einen Befehl testen
{ echo "infra healthcheck"; echo "."; } | ssh forge-runner@server
```

## Tiefenverteidigung

Die 3 Schichten ergänzen sich:

| Schicht | Mechanismus | Schutz |
|---------|-------------|--------|
| 1 | `command=` + `restrict` in `authorized_keys` | Erzwingt die Stufe, blockiert Forwarding/PTY |
| 2 | SSH-Frontière (Login-Shell) | Validiert gegen die TOML-Whitelist |
| 3 | `sudo` in sudoers | Beschränkt Systembefehle |

Selbst wenn ein Angreifer einen SSH-Schlüssel kompromittiert, kann er nur in der Whitelist autorisierte Befehle ausführen. Selbst wenn er Schicht 2 umgeht, werden Privilegien durch sudoers begrenzt.

## Rollback

Falls etwas nicht funktioniert, kehren Sie zur regulären Shell zurück:

```bash
# Über die Konsole (IPMI/KVM) oder ein anderes Admin-Konto
chsh -s /bin/bash forge-runner
```

**Tipp**: Sichern Sie `/etc/passwd` bevor Sie die Login-Shell ändern.

```bash
cp /etc/passwd /etc/passwd.bak.$(date +%Y%m%d)
```

---

**Weiter**: [Erste Schritte](@/guides/premier-usage.md) — Ihr erster SSH-Befehl über SSH-Frontière.
