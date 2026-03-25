+++
title = "Token und Sicherheit"
description = "RBAC-Authentifizierung mit Token in SSH-Frontière konfigurieren"
date = 2026-03-24
weight = 3
+++

# Token und Sicherheit

SSH-Frontière bietet zwei komplementäre Zugriffskontrollmechanismen: die **Basisstufe** (über `authorized_keys`) und die **Token-Erhöhung** (über das Header-Protokoll).

## Basisstufen über authorized_keys

Jeder SSH-Schlüssel hat eine feste Vertrauensstufe, definiert in `authorized_keys`:

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

Diese Stufe ist das **garantierte Minimum**: ein Client mit `--level=read` kann nur auf `read`-Aktionen zugreifen.

## Token-Erhöhung

Ein Client kann sich über seine Basisstufe erheben, indem er sich mit einem Token authentifiziert. Die effektive Stufe wird `max(Basisstufe, Token-Stufe)`.

### Token konfigurieren

```toml
[auth]
challenge_nonce = false    # true für Anti-Replay-Modus

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

### Secret generieren

```bash
# Zufälliges Secret generieren
head -c 32 /dev/urandom | base64
# Ergebnis: etwas wie "dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="

# In config.toml:
# secret = "b64:dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="
```

### Token verwenden

Die Authentifizierung funktioniert je nach Konfiguration in zwei Modi:

**Einfacher Modus** (`challenge_nonce = false`, Standard):

1. Der Client berechnet den Proof: `SHA-256(secret)`
2. Der Client sendet den Header: `+ auth token=runner-ci proof=...`

**Nonce-Modus** (`challenge_nonce = true`):

1. Der Server sendet einen Nonce im Banner: `+> challenge nonce=a1b2c3...`
2. Der Client berechnet den Proof: `SHA-256(XOR_encrypt(secret || nonce, secret))`
3. Der Client sendet den Header: `+ auth token=runner-ci proof=...`

```bash
# Proof mit dem Hilfsbinary berechnen
# Einfacher Modus (ohne Nonce):
PROOF=$(proof --secret "mein-secret")
# Nonce-Modus:
PROOF=$(proof --secret "mein-secret" --nonce "a1b2c3...")

# Mit Authentifizierung senden
{
  echo "+ auth token=runner-ci proof=$PROOF"
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@server
```

## Sichtbarkeits-Tags

Tags filtern den Zugang zu Aktionen horizontal. Ein Token mit dem Tag `forgejo` sieht nur Aktionen mit dem Tag `forgejo`, selbst wenn es die Stufe `ops` hat.

```toml
# Token mit Tags
[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]

# Aktion mit Tags
[domains.forgejo.actions.deploy]
description = "Deployment"
level = "ops"
execute = "sudo /usr/local/bin/deploy.sh {domain}"
tags = ["forgejo", "deploy"]
```

Zugriffsregeln:
- **Aktion ohne Tags**: für alle zugänglich (bei ausreichender Stufe)
- **Aktion mit Tags**: zugänglich wenn mindestens ein Tag mit der Identität gemeinsam ist
- In einer Sitzung addieren sich die Tags mehrerer Token (Vereinigung)

## Anti-Replay-Nonce-Modus

Standardmäßig (`challenge_nonce = false`) ist der Proof ein einfaches `SHA-256(secret)` — kein Nonce. Bei aktiviertem `challenge_nonce = true` sendet der Server einen Nonce im Banner und der Proof integriert diesen Nonce. Der Nonce wird nach jeder erfolgreichen Authentifizierung regeneriert, was das Wiedereinspielen eines abgefangenen Proofs verhindert.

```toml
[auth]
challenge_nonce = true
```

Dieser Modus wird empfohlen für Zugriffe außerhalb von SSH (direktes TCP) oder wenn der Kanal nicht Ende-zu-Ende verschlüsselt ist.

## Schutz vor Missbrauch

| Schutz | Konfiguration | Standard |
|--------|---------------|----------|
| Lockout nach N Fehlschlägen | `max_auth_failures` | 3 |
| IP-Sperre | `ban_command` | deaktiviert |
| Sitzungs-Timeout | `timeout_session` | 3600s |

```toml
[global]
max_auth_failures = 3
ban_command = "/usr/sbin/iptables -A INPUT -s {ip} -j DROP"
```

Nach 3 fehlgeschlagenen Authentifizierungsversuchen wird die Verbindung geschlossen. Wenn `ban_command` konfiguriert ist, wird die Quell-IP gesperrt.

---

**Weiter**: [SSH-Frontière mit KI-Agenten nutzen](@/guides/agents-ia.md) — kontrollierten Zugang für LLMs konfigurieren.
