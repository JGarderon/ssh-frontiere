+++
title = "Tokens et sécurité"
description = "Configurer l'authentification RBAC avec tokens dans SSH-Frontière"
date = 2026-03-24
weight = 3
+++

# Tokens et sécurité

SSH-Frontière propose deux mécanismes de contrôle d'accès complémentaires : le **niveau de base** (via `authorized_keys`) et l'**élévation par token** (via le protocole d'en-têtes).

## Niveaux de base via authorized_keys

Chaque clé SSH a un niveau de confiance fixe, défini dans `authorized_keys` :

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

Ce niveau est le **minimum garanti** : un client avec `--level=read` ne peut accéder qu'aux actions de niveau `read`.

## Élévation par token

Un client peut s'élever au-dessus de son niveau de base en s'authentifiant avec un token. Le niveau effectif devient `max(niveau_base, niveau_token)`.

### Configurer un token

```toml
[auth]
challenge_nonce = false    # true pour le mode anti-replay

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

### Générer un secret

```bash
# Générer un secret aléatoire
head -c 32 /dev/urandom | base64
# Résultat : quelque chose comme "dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="

# Dans config.toml :
# secret = "b64:dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="
```

### Utiliser un token

L'authentification fonctionne en deux modes selon la configuration :

**Mode simple** (`challenge_nonce = false`, par défaut) :

1. Le client calcule le proof : `SHA-256(secret)`
2. Le client envoie l'en-tête : `+ auth token=runner-ci proof=...`

**Mode nonce** (`challenge_nonce = true`) :

1. Le serveur envoie un nonce dans la bannière : `+> challenge nonce=a1b2c3...`
2. Le client calcule le proof : `SHA-256(XOR_encrypt(secret || nonce, secret))`
3. Le client envoie l'en-tête : `+ auth token=runner-ci proof=...`

```bash
# Calculer le proof avec le binaire auxiliaire
# Mode simple (sans nonce) :
PROOF=$(proof --secret "mon-secret")
# Mode nonce :
PROOF=$(proof --secret "mon-secret" --nonce "a1b2c3...")

# Envoyer avec authentification
{
  echo "+ auth token=runner-ci proof=$PROOF"
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@serveur
```

## Tags de visibilité

Les tags filtrent horizontalement l'accès aux actions. Un token avec le tag `forgejo` ne voit que les actions taguées `forgejo`, même s'il a le niveau `ops`.

```toml
# Token avec tags
[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]

# Action avec tags
[domains.forgejo.actions.deploy]
description = "Déploiement"
level = "ops"
execute = "sudo /usr/local/bin/deploy.sh {domain}"
tags = ["forgejo", "deploy"]
```

Règles d'accès :
- **Action sans tags** : accessible par tous (si le niveau est suffisant)
- **Action avec tags** : accessible si au moins un tag est en commun avec l'identité
- En session, les tags de plusieurs tokens s'additionnent (union)

## Mode nonce anti-replay

Par défaut (`challenge_nonce = false`), le proof est un simple `SHA-256(secret)` — pas de nonce. En activant `challenge_nonce = true`, le serveur envoie un nonce dans la bannière et le proof intègre ce nonce. Le nonce est régénéré après chaque authentification réussie, ce qui empêche le rejeu d'un proof intercepté.

```toml
[auth]
challenge_nonce = true
```

Ce mode est recommandé pour les accès hors SSH (TCP direct) ou quand le canal n'est pas chiffré de bout en bout.

## Protection contre les abus

| Protection | Configuration | Défaut |
|------------|---------------|--------|
| Lockout après N échecs | `max_auth_failures` | 3 |
| Ban IP | `ban_command` | désactivé |
| Timeout session | `timeout_session` | 3600s |

```toml
[global]
max_auth_failures = 3
ban_command = "/usr/sbin/iptables -A INPUT -s {ip} -j DROP"
```

Après 3 échecs d'authentification, la connexion est coupée. Si `ban_command` est configuré, l'IP source est bannie.

---

**Suite** : [Utiliser SSH-Frontière avec des agents IA](@/guides/agents-ia.md) — configurer un accès contrôlé pour des LLM.
