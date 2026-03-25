+++
title = "Configuration"
description = "Écrire le fichier config.toml de SSH-Frontière"
date = 2026-03-24
weight = 3
+++

# Configuration

SSH-Frontière utilise un fichier TOML pour déclarer les domaines, actions, niveaux d'accès, arguments et tokens d'authentification.

## Emplacement

**Chemin par défaut** : `/etc/ssh-frontiere/config.toml`

**Override** (par ordre de priorité) :
1. `--config <path>` dans la ligne `command=` de `authorized_keys`
2. Variable d'environnement `SSH_FRONTIERE_CONFIG`
3. Chemin par défaut

**Permissions recommandées** : `root:forge-runner 640` (adaptez le groupe au compte de service utilisé).

## Structure du fichier

```toml
[global]                              # Paramètres généraux
[domains.<id>]                        # Domaines fonctionnels
  [domains.<id>.actions.<id>]         # Actions autorisées
    [domains.<id>.actions.<id>.args]  # Arguments nommés (optionnel)
[auth]                                # Authentification RBAC (optionnel)
  [auth.tokens.<id>]                  # Tokens avec secret, niveau et tags
```

## Section `[global]`

| Clé | Type | Défaut | Description |
|-----|------|--------|-------------|
| `log_file` | string | **obligatoire** | Chemin du fichier de log JSON |
| `default_timeout` | entier | `300` | Timeout par défaut en secondes |
| `max_stdout_chars` | entier | `65536` | Limite stdout (64 Ko) |
| `max_stderr_chars` | entier | `16384` | Limite stderr (16 Ko) |
| `max_output_chars` | entier | `131072` | Hard limit globale (128 Ko) |
| `max_stream_bytes` | entier | `10485760` | Limite volume streaming (10 Mo) |
| `timeout_session` | entier | `3600` | Timeout session keepalive |
| `max_auth_failures` | entier | `3` | Tentatives auth avant lockout |
| `ban_command` | string | `""` | Commande de ban IP (placeholder `{ip}`) |
| `log_comments` | bool | `false` | Journaliser les lignes `#` du client |
| `expose_session_id` | bool | `false` | Afficher l'UUID de session dans la bannière |

Les clés `log_level`, `default_level` et `mask_sensitive` sont acceptées par le parseur pour compatibilité avec d'anciennes configurations, mais ne sont plus utilisées.

## Section `[domains]`

Un **domaine** est un périmètre fonctionnel (ex : `forgejo`, `infra`, `notify`). Chaque domaine contient des **actions** autorisées.

```toml
[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Sauvegarde la configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
max_body_size = 65536       # Limite body (64 Ko, optionnel)
```

Chaque action accepte les clés suivantes : `description` (obligatoire), `level` (obligatoire), `execute` (obligatoire), `timeout` (optionnel, override du global), `tags` (optionnel), `max_body_size` (optionnel, défaut 65536 octets — limité pour le protocole `+body`).

### Niveaux de confiance

Hiérarchie stricte : `read` < `ops` < `admin`

| Niveau | Usage |
|--------|-------|
| `read` | Consultation : healthcheck, status, list |
| `ops` | Opérations courantes : backup, deploy, restart |
| `admin` | Toutes les actions + administration |

### Arguments

Les arguments sont déclarés comme un dictionnaire TOML :

```toml
[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }
```

| Champ | Type | Description |
|-------|------|-------------|
| `type` | string | `"enum"` ou `"string"` |
| `values` | liste | Valeurs autorisées (pour `enum`) |
| `default` | string | Valeur par défaut (rend l'argument optionnel) |
| `sensitive` | bool | Si `true`, masque dans les logs |
| `free` | bool | Si `true`, accepte toute valeur sans contrainte |

### Placeholders dans `execute`

| Placeholder | Description |
|-------------|-------------|
| `{domain}` | Nom du domaine (toujours disponible) |
| `{nom_arg}` | Valeur de l'argument correspondant |

### Tags de visibilité

Les tags filtrent horizontalement l'accès aux actions. Une action sans tags est accessible par tous. Une action avec tags n'est accessible qu'aux identités dont les tags ont au moins un tag en commun.

```toml
[domains.forgejo.actions.deploy]
# ...
tags = ["forgejo", "deploy"]
```

## Section `[auth]` (optionnelle)

L'authentification RBAC permet l'élévation de privilèges via challenge-response :

```toml
[auth]
challenge_nonce = false              # true = mode nonce anti-replay

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Secret en base64
level = "ops"                               # Niveau accordé
tags = ["forgejo"]                          # Tags de visibilité
```

Le secret doit être préfixé par `b64:` et encodé en base64. Pour générer un secret :

```bash
echo -n "mon-secret-aleatoire" | base64
# bW9uLXNlY3JldC1hbGVhdG9pcmU=
```

## Validation au chargement

La configuration est validée intégralement à chaque chargement (fail-fast). En cas d'erreur, le programme s'arrête avec le code 129. Validations :

- Syntaxe TOML correcte
- Au moins un domaine, au moins une action par domaine
- Chaque action a un `execute` et un `level` valide
- Les placeholders `{arg}` dans `execute` correspondent aux arguments déclarés
- Les arguments enum ont au moins une valeur autorisée
- Les valeurs par défaut sont dans la liste des valeurs autorisées
- `max_stdout_chars` et `max_stderr_chars` <= `max_output_chars`

## Exemple complet

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
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Sauvegarde la configuration Forgejo"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"

[domains.forgejo.actions.deploy]
description = "Déploiement avec tag de version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.infra]
description = "Infrastructure serveur"

[domains.infra.actions.healthcheck]
description = "Vérification de santé"
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

Pour un guide détaillé avec tous les cas d'usage, consultez le [guide de configuration complet](https://github.com/nothus-forge/ssh-frontiere/blob/main/docs/references/configuration.md) dans le dépôt.

---

**Suite** : [Déploiement](@/installation/deploiement.md) — mettre en production.
