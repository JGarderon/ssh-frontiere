# SSH Frontière v3.0.0

**Website**: [pages.nothus.fr/ssh-frontiere](https://pages.nothus.fr/ssh-frontiere/)

[![License: EUPL-1.2](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.md)

Login shell SSH restreint en Rust — point d'entrée unique pour toutes les connexions SSH entrantes sur un serveur.

SSH Frontière remplace le shell classique (`/bin/bash`) dans `/etc/passwd` et agit comme un **dispatcher sécurisé** : il valide chaque commande SSH contre une whitelist TOML, applique un contrôle d'accès RBAC à 3 niveaux, et renvoie les résultats en JSON structuré via un protocole d'en-têtes sur stdin/stdout.

## Vocation

SSH Frontière est un **composant de sécurité** conçu pour les comptes de service SSH :

- **Runners Forgejo Actions** : exécution d'opérations d'infrastructure depuis des conteneurs
- **Agents IA (LLM)** : accès contrôlé à des ressources serveur avec niveaux de confiance
- **Maintenance automatisée** : backups, déploiements, healthchecks

Le programme est **synchrone et one-shot** : SSH crée un nouveau processus pour chaque connexion, le dispatcher valide et exécute, puis meurt. Pas de démon, pas d'async, pas de Tokio.

## Installation

### Prérequis

- Rust 1.70+ avec la cible `x86_64-unknown-linux-musl`
- `make` (optionnel, pour les raccourcis)

### Compilation

```bash
# Via make
make release

# Ou directement
cargo build --release --target x86_64-unknown-linux-musl
```

Le binaire statique résultant (`target/x86_64-unknown-linux-musl/release/ssh-frontiere`, ~1-2 Mo) est déployable sans aucune dépendance système.

```bash
sudo cp target/x86_64-unknown-linux-musl/release/ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere
```

## Configuration TOML

Fichier par défaut : `/etc/ssh-frontiere/config.toml`.
Override : `--config <path>` ou variable `SSH_FRONTIERE_CONFIG`.

### Exemple complet

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300          # Timeout par défaut (secondes)
default_level = "read"         # Niveau RBAC par défaut
mask_sensitive = true           # Masque les arguments sensibles dans les logs
max_stdout_chars = 65536       # Limite stdout capturé
max_stderr_chars = 16384       # Limite stderr capturé
max_output_chars = 131072      # Limite dure globale
timeout_session = 3600         # Timeout session keepalive (secondes)
max_auth_failures = 3          # Tentatives auth avant lockout
log_comments = false           # Logger les commentaires client
ban_command = ""               # Commande ban IP (ex: "/usr/sbin/iptables -A INPUT -s {ip} -j DROP")

# --- Authentification RBAC (optionnel) ---

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Secret en base64 avec préfixe b64:
level = "ops"                                # Niveau accordé par ce token

[auth.tokens.admin-deploy]
secret = "b64:c2VjcmV0LWFkbWluLWRlcGxveQ=="
level = "admin"

# --- Domaines et actions ---

[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Sauvegarde la configuration Forgejo"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
args = []

[domains.forgejo.actions.deploy]
description = "Déploiement d'une version"
level = "admin"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {version}"

[[domains.forgejo.actions.deploy.args]]
name = "version"
type = "enum"
values = ["latest", "stable", "canary"]

[domains.infra]
description = "Infrastructure serveur"

[domains.infra.actions.healthcheck]
description = "Vérification de santé"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"
args = []

[domains.infra.actions.set-password]
description = "Changement de mot de passe service"
level = "admin"
timeout = 30
execute = "sudo /usr/local/bin/set-password.sh {password}"

[[domains.infra.actions.set-password.args]]
name = "password"
type = "string"
sensitive = true    # Masqué dans les logs si mask_sensitive = true
```

### Types d'arguments

| Type | Description | Validation |
|------|-------------|------------|
| `string` | Texte libre | Longueur max 256 caractères |
| `enum` | Valeur parmi une liste | Doit correspondre à une valeur de `values` |

### Placeholders dans `execute`

- `{domain}` : remplacé par le nom du domaine (toujours disponible)
- `{nom_arg}` : remplacé par la valeur de l'argument correspondant

## Déploiement

### 1. Login shell (`/etc/passwd`)

```bash
# Créer le compte de service
sudo useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

Le programme sera invoqué directement par `sshd` en tant que login shell.

### 2. Clés SSH avec `authorized_keys`

```
# ~forge-runner/.ssh/authorized_keys

# Clé runner CI (niveau ops)
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-key

# Clé monitoring (niveau read seul)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitor-key

# Clé admin (niveau admin)
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

L'option `command=` force l'exécution de ssh-frontiere avec le `--level` choisi, quelle que soit la commande envoyée par le client. L'option `restrict` désactive le forwarding de port, l'agent forwarding, le PTY, et les X11.

### 3. Sudoers (couche 3)

```
# /etc/sudoers.d/ssh-frontiere
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/set-password.sh
```

Seules les commandes listées dans la whitelist TOML **et** autorisées dans sudoers pourront s'exécuter avec des privilèges.

## Protocole d'en-têtes

SSH Frontière utilise un protocole texte sur stdin/stdout avec 4 préfixes (ADR 0006).

### Préfixes

| Préfixe | Rôle | Direction |
|---------|------|-----------|
| `+` | **Configure** : directives (`capabilities`, `challenge`, `auth`, `session`) | bidirectionnel |
| `#` | **Commente** : information, bannière, messages | bidirectionnel |
| `$` | **Ordonne** : commande à exécuter | client -> serveur |
| `>` | **Répond** : réponse JSON | serveur -> client |

### Flux de connexion

```
CLIENT                              SERVEUR
  |                                    |
  |  <-- bannière + capabilities ----  |   # ssh-frontiere 0.1.0
  |  <-- challenge nonce ---------- -  |   + capabilities rbac, session, help
  |                                    |   + challenge nonce=a1b2c3...
  |                                    |   # type "$ help" for available commands
  |                                    |
  |  --- +auth (optionnel) -------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (optionnel) ----->   |   + session keepalive
  |  --- # commentaire (opt.) ----->   |   # client-id: forgejo-runner-12
  |  --- ligne vide --------------->   |   (fin des en-têtes)
  |                                    |
  |  --- $ commande --------------->   |   $ forgejo backup-config
  |  <-- > réponse JSON -----------    |   > {"status_code":0,"status_message":"executed",...}
  |                                    |
  |  (si +session keepalive)           |
  |  --- $ commande 2 ------------>   |   $ infra healthcheck
  |  <-- > réponse JSON 2 ---------   |   > {"status_code":0,...}
  |  --- $ exit ------------------->   |   $ exit
  |  <-- # session closed ----------   |   # session closed
  |                                    |
```

### Réponse JSON (4 champs)

Chaque commande produit une réponse `>` contenant un objet JSON :

```json
{
  "status_code": 0,
  "status_message": "executed",
  "stdout": "Backup completed\n",
  "stderr": ""
}
```

- `stdout`/`stderr` = `null` : commande non exécutée (rejet, erreur config)
- `stdout`/`stderr` = `""` : commande exécutée sans sortie

### Codes de sortie

| Code | Signification |
|------|---------------|
| 0 | Succès |
| 1-127 | Code de la commande enfant (passthrough) |
| 128 | Commande refusée |
| 129 | Erreur de configuration |
| 130 | Timeout |
| 131 | Niveau RBAC insuffisant |
| 132 | Erreur de protocole |

## Exemples concrets

### Mode one-shot (sans protocole d'en-têtes)

Pour une utilisation simple compatible avec les scripts existants, le client envoie directement les en-têtes minimales puis la commande :

```bash
# Depuis le client SSH, en mode pipe :
{
  echo ""              # Ligne vide = fin des en-têtes (aucun header)
  echo "$ infra healthcheck"
} | ssh forge-runner@serveur
```

### Mode session (keepalive)

Le mode session permet d'envoyer plusieurs commandes dans une seule connexion SSH :

```bash
{
  echo "+ session keepalive"
  echo ""
  echo "$ infra healthcheck"
  echo "$ forgejo backup-config"
  echo "$ exit"
} | ssh forge-runner@serveur
```

Le serveur répond avec une ligne `>` JSON pour chaque commande `$`.

### Authentification RBAC (élévation de niveau)

Un client avec `--level=read` peut s'élever à `ops` ou `admin` via le challenge-response :

```bash
{
  echo "+ auth token=runner-ci proof=<hmac-sha256-hex>"
  echo ""
  echo "$ forgejo backup-config"    # Requiert ops, autorisé grâce au token
} | ssh forge-runner@serveur
```

Le `proof` est le HMAC-SHA256 du nonce (envoyé par le serveur dans `+ challenge nonce=...`) avec le secret partagé du token. Le niveau effectif est `max(--level, token.level)`.

### Découverte (help / list)

```bash
# Liste complète des commandes accessibles
{ echo ""; echo "$ help"; } | ssh forge-runner@serveur

# Détails d'un domaine
{ echo ""; echo "$ help forgejo"; } | ssh forge-runner@serveur

# Liste courte (domaine + action + description)
{ echo ""; echo "$ list"; } | ssh forge-runner@serveur
```

Les commandes `help` et `list` ne montrent que les actions accessibles au niveau effectif du client.

## Sécurité

### Trois couches de défense en profondeur

| Couche | Mécanisme | Protection |
|--------|-----------|------------|
| 1 | `command=` + `restrict` dans `authorized_keys` | Force le `--level`, bloque forwarding/PTY |
| 2 | `ssh-frontiere` (login shell) | Valide la commande contre la whitelist TOML |
| 3 | `sudo` whitelist dans sudoers | Restreint les commandes système privilégiées |

Même si un attaquant contourne la couche 1 (clé compromise), la couche 2 bloque toute commande hors whitelist. La couche 3 limite les privilèges système.

### Parseur grammatical, pas liste noire

**ssh-frontiere n'est pas un shell.** La sécurité repose sur un **parseur grammatical**, pas sur un filtrage de caractères.

- La grammaire attendue est `domaine action [args]` — tout ce qui ne respecte pas cette structure est rejeté
- Les caractères spéciaux (`|`, `;`, `&`, `$`, etc.) entre guillemets sont du **contenu** d'argument, pas de la syntaxe shell — ils sont valides
- Il n'y a pas de « caractères interdits » — il y a une grammaire, et tout ce qui ne la respecte pas est rejeté
- `std::process::Command` exécute directement sans shell intermédiaire — aucune injection possible

### Ce que le programme ne fait JAMAIS

- Invoquer un shell (`/bin/bash`, `/bin/sh`)
- Accepter du pipe, de la redirection, du chaînage (`|`, `>`, `&&`, `;`)
- Exécuter une commande non listée dans la whitelist
- Donner accès à un TTY interactif

### Protections supplémentaires

- **Timeout** par commande avec kill du process group (SIGTERM puis SIGKILL)
- **Lockout** après N tentatives d'auth échouées (configurable, défaut : 3)
- **Ban IP** optionnel via commande externe configurable (`ban_command`)
- **Masquage** des arguments sensibles dans les logs JSON
- **Limite de taille** sur les sorties capturées (stdout, stderr)
- **Nonce anti-replay** régénéré après chaque authentification réussie en session
- **env_clear()** sur les processus enfants (seul `PATH` est préservé)

## Tests

```bash
# Tests unitaires et d'intégration
make test

# Tests end-to-end SSH (Docker requis)
make e2e

# Lints (fmt + clippy)
make lint

# Audit de sécurité des dépendances
make audit
```

Les tests E2E (`make e2e`) démarrent un environnement Docker compose avec un serveur SSH et un client, puis exécutent des scénarios couvrant le protocole (PRO-*), l'authentification (AUT-*), les sessions (SES-*), la sécurité (SEC-*), la robustesse (ROB-*) et le logging (LOG-*).

## Contribuer

Les contributions sont les bienvenues ! Consultez le [guide de contribution](CONTRIBUTING.md) pour les détails.

## Licence

Ce projet est distribué sous la [Licence Publique de l'Union européenne (EUPL-1.2)](LICENSE.md).

Copyright (c) Julien Garderon, 2024-2026
