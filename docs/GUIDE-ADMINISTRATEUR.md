# Guide administrateur — SSH Frontière v3

**Version** : 3.x
**Dernière mise à jour** : 2026-03-25

Pour plus d'informations (présentation, sécurité, FAQ, alternatives) : **https://pages.nothus.fr/ssh-frontiere/**

---

## Table des matières

1. [Installation](#1-installation)
2. [Configuration (config.toml)](#2-configuration-configtoml)
3. [Protocole](#3-protocole)
4. [Authentification RBAC](#4-authentification-rbac)
5. [Scripts d'opérations](#5-scripts-dopérations)
6. [Chaînage de commandes](#6-chaînage-de-commandes)
7. [Maintenance](#7-maintenance)
8. [Dépannage](#8-dépannage)

---

## 1. Installation

### 1.1 Prérequis

- Serveur Linux avec `sshd` configuré
- Aucune dépendance runtime : le binaire est compilé en statique (`x86_64-unknown-linux-musl`)
- Taille du binaire : ~1-2 Mo

### 1.2 Déploiement du binaire

```bash
# Copier le binaire sur le serveur
scp target/x86_64-unknown-linux-musl/release/ssh-frontiere root@serveur:/usr/local/bin/

# Rendre exécutable
chmod 755 /usr/local/bin/ssh-frontiere
```

### 1.3 Création du compte de service

Créer un utilisateur système dont le login shell est `ssh-frontiere` :

```bash
useradd -r -m -s /usr/local/bin/ssh-frontiere svc-ssh
```

Ou modifier un compte existant :

```bash
chsh -s /usr/local/bin/ssh-frontiere svc-ssh
```

Résultat dans `/etc/passwd` :

```
svc-ssh:x:1001:1001::/home/svc-ssh:/usr/local/bin/ssh-frontiere
```

### 1.4 Configuration `authorized_keys`

Chaque clé SSH est associée à un niveau de confiance via l'option `command=` et aux restrictions via `restrict` :

```
# /home/svc-ssh/.ssh/authorized_keys

# Runner Forgejo — niveau ops
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner@forgejo

# Agent IA — niveau read (lecture seule)
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... agent@llm

# Administrateur — niveau admin
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin@ops
```

L'option `restrict` désactive le port forwarding, l'agent forwarding, le PTY et la redirection X11. L'option `command=` force l'exécution de `ssh-frontiere` avec le `--level` spécifié, quelle que soit la commande demandée par le client.

### 1.5 Configuration TOML

```bash
mkdir -p /etc/ssh-frontiere
cp config.toml /etc/ssh-frontiere/config.toml
chmod 644 /etc/ssh-frontiere/config.toml
```

Le chemin par défaut est `/etc/ssh-frontiere/config.toml`. Il peut être changé via :

- L'argument `--config=/chemin/vers/config.toml` dans `command=` de `authorized_keys`
- La variable d'environnement `SSH_FRONTIERE_CONFIG`

Exemple avec chemin personnalisé dans `authorized_keys` :

```
command="/usr/local/bin/ssh-frontiere --level=ops --config=/etc/ssh-frontiere/runner.toml",restrict ssh-ed25519 AAAA...
```

### 1.6 Validation de la configuration

```bash
ssh-frontiere --check-config --config=/etc/ssh-frontiere/config.toml
# Sortie : Config OK: /etc/ssh-frontiere/config.toml
```

En cas d'erreur, le message indique la nature du problème (fichier introuvable, TOML invalide, erreur de validation sémantique).

### 1.7 Création du répertoire de logs

```bash
mkdir -p /var/log/ssh-frontiere
chown svc-ssh:svc-ssh /var/log/ssh-frontiere
chmod 750 /var/log/ssh-frontiere
```

### 1.8 Permissions des fichiers

| Fichier | Propriétaire | Permissions |
|---------|-------------|-------------|
| `/usr/local/bin/ssh-frontiere` | root:root | 755 |
| `/etc/ssh-frontiere/config.toml` | root:root | 644 |
| `/home/svc-ssh/.ssh/authorized_keys` | svc-ssh:svc-ssh | 600 |
| `/var/log/ssh-frontiere/` | svc-ssh:svc-ssh | 750 |

---

## 2. Configuration (config.toml)

### 2.1 Structure générale

```toml
[global]
# Paramètres globaux

[auth]
# Authentification RBAC (optionnel)

[auth.tokens.<nom-du-token>]
# Définition d'un token

[domains.<nom-du-domaine>]
# Définition d'un domaine

[domains.<nom-du-domaine>.actions.<nom-de-laction>]
# Définition d'une action

[domains.<nom-du-domaine>.actions.<nom-de-laction>.args]
# Arguments de l'action (optionnel)
```

### 2.2 Section `[global]`

| Paramètre | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `log_file` | string | *(obligatoire)* | Chemin du fichier de logs JSON |
| `default_timeout` | entier | `300` | Timeout par défaut des commandes (secondes) |
| `max_stdout_chars` | entier | `65536` | Limite stdout par commande (caractères) |
| `max_stderr_chars` | entier | `16384` | Limite stderr par commande (caractères) |
| `max_output_chars` | entier | `131072` | Limite totale stdout+stderr (hard limit, doit être ≥ max_stdout et max_stderr) |
| `timeout_session` | entier | `3600` | Durée maximale d'une session keepalive (secondes) |
| `max_auth_failures` | entier | `3` | Tentatives d'authentification avant verrouillage |
| `log_comments` | booléen | `false` | Journaliser les commentaires `#` envoyés par le client |
| `ban_command` | string | `""` | Commande à exécuter lors d'un verrouillage (placeholder `{ip}`) |
| `expose_session_id` | booléen | `false` | Afficher l'UUID de session dans la bannière |
| `max_stream_bytes` | entier | `10485760` | Limite totale du streaming stdout+stderr (10 Mo) |

Exemple complet :

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300
max_stdout_chars = 65536
max_stderr_chars = 16384
max_output_chars = 131072
timeout_session = 3600
max_auth_failures = 3
log_comments = false
ban_command = "/usr/local/bin/fail2ban-client set sshd banip {ip}"
expose_session_id = false
max_stream_bytes = 10485760
```

### 2.3 Section `[auth]` (optionnelle)

Si cette section est absente, aucune authentification par token n'est disponible. Le niveau d'accès est alors uniquement déterminé par `--level` dans `authorized_keys`.

| Paramètre | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `challenge_nonce` | booléen | `false` | Activer le mode challenge-response (nonce aléatoire) |

#### Tokens `[auth.tokens.<nom>]`

| Paramètre | Type | Description |
|-----------|------|-------------|
| `secret` | string | Secret en base64 avec préfixe `b64:` (obligatoire) |
| `level` | string | Niveau RBAC du token : `read`, `ops` ou `admin` |
| `tags` | liste | Tags associés au token (filtrage d'accès) |

Le nom du token doit être alphanumérique avec tirets uniquement.

Exemple :

```toml
[auth]
challenge_nonce = true

[auth.tokens.runner-ci]
secret = "b64:bW9uLXNlY3JldC1ydW5uZXI="
level = "ops"
tags = ["deploy", "monitoring"]

[auth.tokens.admin-ops]
secret = "b64:bW9uLXNlY3JldC1hZG1pbg=="
level = "admin"
tags = ["deploy", "monitoring", "admin"]
```

Pour générer un secret en base64 :

```bash
echo -n "mon-secret-runner" | base64
# bW9uLXNlY3JldC1ydW5uZXI=

# Dans le config.toml : secret = "b64:bW9uLXNlY3JldC1ydW5uZXI="
```

### 2.4 Section `[domains]`

Chaque domaine est un périmètre fonctionnel contenant des actions.

```toml
[domains.infra]
description = "Gestion de l'infrastructure"
```

Un domaine doit contenir au moins une action. La configuration est rejetée si un domaine est vide.

### 2.5 Actions `[domains.<domaine>.actions.<action>]`

| Paramètre | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `description` | string | *(obligatoire)* | Description humaine de l'action |
| `level` | string | *(obligatoire)* | Niveau minimum requis : `read`, `ops` ou `admin` |
| `timeout` | entier | *(global)* | Timeout spécifique à l'action (secondes) |
| `execute` | string | *(obligatoire)* | Commande à exécuter (avec placeholders) |
| `tags` | liste | `[]` | Tags de l'action (si vide : action publique) |
| `max_body_size` | entier | `65536` | Taille maximale du body en octets (64 Ko par défaut) |

Exemple :

```toml
[domains.infra.actions.healthcheck]
description = "Vérifier l'état des services"
level = "read"
timeout = 30
execute = "/opt/scripts/healthcheck.sh"

[domains.infra.actions.deploy]
description = "Déployer une version"
level = "ops"
timeout = 600
execute = "/opt/scripts/deploy.sh {service} {version}"
tags = ["deploy"]

[domains.infra.actions.deploy.args]
service = { type = "enum", values = ["api", "web", "worker"] }
version = { type = "string" }
```

### 2.6 Arguments `[domains.<domaine>.actions.<action>.args]`

Les arguments sont nommés et passés en syntaxe `clé=valeur`. Chaque argument est défini avec :

| Paramètre | Type | Défaut | Description |
|-----------|------|--------|-------------|
| `type` | string | `""` | Type de l'argument (`enum`, `string`, etc.) |
| `values` | liste | *(aucune)* | Valeurs autorisées (obligatoire si `type = "enum"`) |
| `default` | string | *(aucune)* | Valeur par défaut (si absent : argument obligatoire) |
| `sensitive` | booléen | `false` | Marquer la valeur comme sensible dans les logs |
| `free` | booléen | `false` | Texte libre : accepte toute valeur sans contrainte |

Exemples :

```toml
# Argument enum avec valeurs autorisées
service = { type = "enum", values = ["api", "web", "worker"] }

# Argument obligatoire (pas de default)
version = { type = "string" }

# Argument optionnel avec valeur par défaut
env = { type = "enum", values = ["staging", "production"], default = "staging" }

# Argument texte libre (accepte toute valeur)
message = { type = "string", free = true }

# Argument sensible (masqué dans les logs)
token = { type = "string", sensitive = true }
```

### 2.7 Placeholders dans `execute`

La commande `execute` supporte les placeholders suivants :

- `{domain}` : remplacé par le nom du domaine
- `{nom_argument}` : remplacé par la valeur de l'argument correspondant

Chaque placeholder dans `execute` doit avoir un argument correspondant défini (sauf `{domain}`). La configuration est rejetée si un placeholder n'a pas de correspondance.

```toml
# {domain} vaut "infra", {service} et {version} sont des arguments
execute = "/opt/scripts/deploy.sh --domain={domain} --svc={service} --ver={version}"
```

---

## 3. Protocole

SSH Frontière utilise un protocole texte ligne par ligne sur stdin/stdout. Quatre préfixes structurent la communication :

| Préfixe | Direction | Signification |
|---------|-----------|---------------|
| `+` | client → serveur | Configuration (directive) |
| `#` | client → serveur | Commentaire |
| `+>` | serveur → client | Configuration serveur |
| `#>` | serveur → client | Commentaire serveur |
| `>>` | serveur → client | Sortie stdout (streaming) |
| `>>!` | serveur → client | Sortie stderr (streaming) |
| `>>>` | serveur → client | Réponse JSON finale |
| `.` | bidirectionnel | Fin de bloc |

### 3.1 Flux d'une connexion

```
Client                              Serveur
  |                                    |
  |  <--- bannière serveur ----------- |  #> ssh-frontiere 3.x.x
  |  <--- capabilities --------------- |  +> capabilities rbac, session, help, body
  |  <--- challenge (si auth) -------- |  +> challenge nonce=<hex>
  |  <--- info ----------------------- |  #> type "help" for available commands
  |                                    |
  |  --- + auth token=T proof=P ----> |  (optionnel)
  |  --- + session keepalive -------> |  (optionnel)
  |  --- + body ----------------------> |  (optionnel)
  |  --- # commentaire --------------> |  (optionnel)
  |                                    |
  |  --- domaine action clé=val ----> |  (commande)
  |  --- . --------------------------> |  (fin du bloc commande)
  |                                    |
  |  (si +body déclaré)               |
  |  --- contenu du body ------------> |
  |  --- . --------------------------> |  (fin du body, mode défaut)
  |                                    |
  |  <--- >> ligne stdout ------------ |  (streaming)
  |  <--- >>! ligne stderr ----------- |  (streaming)
  |  <--- >>> {"command":...} -------- |  (réponse JSON finale)
  |                                    |
  |  (si session keepalive)           |
  |  --- prochaine commande ---------> |  (boucle)
  |  --- . --------------------------> |  (fin de connexion)
```

### 3.2 Bannière serveur

À chaque connexion, le serveur envoie automatiquement :

```
#> ssh-frontiere 3.x.x
+> capabilities rbac, session, help, body
+> challenge nonce=<32-chars-hex>
+> session <uuid>
#> type "help" for available commands
```

- **capabilities** : liste des fonctionnalités disponibles. `rbac` n'apparaît que si `[auth]` est configuré.
- **challenge** : nonce aléatoire pour l'authentification (uniquement si `challenge_nonce = true` et des tokens existent).
- **session** : UUID v4 de session (uniquement si `expose_session_id = true`).

### 3.3 En-têtes client

Avant d'envoyer une commande, le client peut envoyer des directives :

```
+ auth token=runner-ci proof=<64-chars-hex>
+ session keepalive
+ body
+ body size=1024
+ body stop="END"
+ body size=1024 stop="END"
# Ceci est un commentaire
```

- **`+ auth`** : authentification par token (voir section 4).
- **`+ session keepalive`** : activer le mode session (connexion persistante).
- **`+ body`** : déclarer un body à envoyer après le bloc commande (3 modes, voir section 3.6).
- **`#`** : commentaire libre (journalisé si `log_comments = true`).

La phase d'en-têtes se termine implicitement lorsque le serveur reçoit une ligne de texte (la première ligne de commande).

### 3.4 Format des commandes

```
domaine action clé=valeur clé2=valeur2
.
```

La commande suit la syntaxe : `<domaine> <action> [clé=valeur ...]`, terminée par une ligne contenant uniquement `.`.

Les valeurs contenant des espaces doivent être entre guillemets (doubles `"` ou simples `'`) :

```
infra deploy service=api version="1.2.3 beta"
.
```

Les caractères spéciaux dans les guillemets sont du contenu, pas de la syntaxe shell. Il n'y a pas de liste noire de caractères — la sécurité repose sur le parseur grammatical.

### 3.5 Réponse JSON finale (`>>>`)

Chaque commande produit une réponse JSON finale sur une ligne préfixée `>>>` :

```json
>>> {"command":"infra deploy service=api version=1.2.3","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

| Champ | Type | Description |
|-------|------|-------------|
| `command` | string | Commande telle que reçue |
| `status_code` | entier | Code de sortie (0 = succès) |
| `status_message` | string | Message descriptif |
| `stdout` | string\|null | Contenu stdout (null en mode streaming) |
| `stderr` | string\|null | Contenu stderr (null en mode streaming) |

En mode streaming (comportement par défaut), stdout et stderr sont envoyés ligne par ligne via `>>` et `>>!` avant la réponse `>>>`. Les champs `stdout` et `stderr` de la réponse finale sont alors `null`.

### 3.6 Protocole body

Le body permet d'envoyer du contenu volumineux (données, fichiers) au processus enfant via son stdin. Il est déclaré dans les en-têtes par `+ body` et lu après le terminateur `.` du bloc commande.

**Trois modes de délimitation :**

| Mode | Directive | Terminaison |
|------|-----------|-------------|
| Défaut | `+ body` | Ligne contenant uniquement `.` |
| Taille exacte | `+ body size=N` | Après N octets lus |
| Sentinelle | `+ body stop="MOT"` | Ligne contenant uniquement `MOT` |
| Combiné | `+ body size=N stop="MOT"` | Le premier atteint termine |

Exemple avec le mode défaut :

```
+ body
infra import-config service=api
.
clé1=valeur1
clé2=valeur2
.
```

Exemple avec le mode taille :

```
+ body size=42
infra import-config service=api
.
{"key": "value", "count": 12345678901}
```

La taille maximale du body est définie par `max_body_size` dans l'action (défaut : 64 Ko). Si le body dépasse cette limite, la connexion est coupée avec une erreur de protocole.

### 3.7 Codes de sortie

| Code | Signification |
|------|---------------|
| `0` | Succès |
| `1-127` | Code de sortie du processus enfant |
| `128` | Commande rejetée (domaine/action inconnu, syntaxe invalide, argument invalide) |
| `129` | Erreur de configuration (fichier introuvable, TOML invalide) |
| `130` | Timeout (commande dépassant le délai autorisé) |
| `131` | Niveau insuffisant (RBAC) |
| `132` | Erreur de protocole (ligne trop longue, format invalide) |
| `133` | Erreur stdin/body (le processus enfant a fermé stdin avant la fin de l'écriture du body) |

### 3.8 Commandes intégrées

| Commande | Description | Format de sortie |
|----------|-------------|-----------------|
| `help` | Vue d'ensemble des domaines et actions accessibles | Texte via `#>` + réponse `>>>` |
| `help <domaine>` | Détail d'un domaine (actions, arguments, niveaux) | Texte via `#>` + réponse `>>>` |
| `list` | Liste JSON des actions accessibles | JSON via `>>>` (champ `stdout`) |
| `exit` | Terminer la session (mode session) | Réponse `>>>` avec code 0 |

---

## 4. Authentification RBAC

### 4.1 Niveaux de confiance

Trois niveaux ordonnés :

```
read < ops < admin
```

| Niveau | Usage typique |
|--------|--------------|
| `read` | Lecture seule, healthchecks, statuts |
| `ops` | Opérations courantes, déploiements, backups |
| `admin` | Administration système, configuration |

Le **niveau de base** est défini par `--level` dans `authorized_keys`. Le **niveau effectif** est le maximum entre le niveau de base et le niveau du token authentifié :

```
niveau_effectif = max(niveau_base, niveau_token)
```

### 4.2 Tags

Les tags permettent un contrôle d'accès plus fin que les niveaux seuls :

- **Action sans tags** = action publique (accessible à tout niveau suffisant).
- **Action avec tags** = nécessite qu'au moins un tag effectif corresponde.
- Les tags du token sont fusionnés dans les tags effectifs lors de l'authentification.
- Les tags sont normalisés en minuscules, dédupliqués et triés.

Exemple :

```toml
# Action accessible uniquement aux tokens ayant le tag "deploy"
[domains.infra.actions.deploy]
level = "ops"
tags = ["deploy"]
execute = "/opt/scripts/deploy.sh"

# Token avec le tag "deploy"
[auth.tokens.runner-ci]
secret = "b64:..."
level = "ops"
tags = ["deploy"]
```

### 4.3 Authentification sans nonce (mode simple)

Si `challenge_nonce = false` (ou absent) :

1. Le client calcule le proof : `SHA-256(secret_brut)`
2. Le client envoie : `+ auth token=<nom> proof=<hex-sha256>`

Calcul du proof :

```bash
# Secret brut du token
SECRET="mon-secret-runner"

# Proof = SHA-256 du secret brut
echo -n "$SECRET" | sha256sum | cut -d' ' -f1
# ou avec ssh-frontiere-proof :
ssh-frontiere-proof --secret "$SECRET"
```

### 4.4 Authentification avec nonce (mode challenge-response)

Si `challenge_nonce = true` :

1. Le serveur envoie un nonce aléatoire dans la bannière : `+> challenge nonce=<hex>`
2. Le client calcule le proof : `SHA-256(XOR_encrypt(secret || nonce, secret))`
3. Le client envoie : `+ auth token=<nom> proof=<hex-sha256>`

Le XOR cipher utilise un keystream SHA-256 CTR : `SHA-256(secret || 0x00) || SHA-256(secret || 0x01) || ...`

Calcul du proof avec l'utilitaire intégré :

```bash
# Le nonce est lu depuis la bannière du serveur
NONCE="<hex-nonce-depuis-la-banniere>"
SECRET="mon-secret-runner"

ssh-frontiere-proof --secret "$SECRET" --nonce "$NONCE"
```

En mode session, un nouveau nonce est généré après chaque authentification réussie (protection contre le rejeu). Le serveur l'annonce via `#> new challenge nonce=<hex>`.

### 4.5 Verrouillage

Après `max_auth_failures` tentatives échouées (défaut : 3), la session est terminée. Si `ban_command` est configuré, elle est exécutée avec l'IP du client :

```toml
ban_command = "/usr/local/bin/fail2ban-client set sshd banip {ip}"
```

L'IP est extraite de la variable d'environnement `SSH_CLIENT` (définie par sshd).

---

## 5. Scripts d'opérations

### 5.1 Convention

Chaque action exécute une commande définie dans `execute`. En pratique, on pointe vers un script dédié :

```toml
[domains.backup.actions.run]
description = "Lancer un backup"
level = "ops"
timeout = 3600
execute = "/opt/scripts/backup/run.sh {target}"

[domains.backup.actions.run.args]
target = { type = "enum", values = ["database", "files", "all"] }
```

### 5.2 Environnement d'exécution

Le processus enfant est exécuté avec :

- **PATH restreint** : `/usr/local/bin:/usr/bin:/bin`
- **Environnement vidé** : `env_clear()` — seuls `PATH` et `SSH_FRONTIERE_SESSION` sont définis
- **Pas de shell** : exécution directe via `execve` (pas de `/bin/sh -c`)
- **Process group isolé** : le processus et ses enfants sont dans un groupe séparé
- **stdin** : `/dev/null` (sauf si un body est fourni)
- **stdout/stderr** : capturés et streamés ligne par ligne

La variable `SSH_FRONTIERE_SESSION` contient l'UUID v4 de la session en cours.

### 5.3 Placeholders

Le template `execute` est découpé par espaces. Chaque token est substitué individuellement :

```toml
execute = "/opt/scripts/deploy.sh {service} {version}"
# Avec service=api et version=1.2.3 :
# → ["/opt/scripts/deploy.sh", "api", "1.2.3"]
```

Un argument `free = true` contenant des espaces reste un seul token après substitution (pas de découpage).

### 5.4 Body via stdin

Si le client a déclaré `+ body`, le contenu du body est écrit sur le stdin du processus enfant. Le script peut le lire normalement :

```bash
#!/bin/bash
# /opt/scripts/import-config.sh
# Lit le body depuis stdin
cat > /tmp/import-config.json
jq . /tmp/import-config.json
```

### 5.5 Sudoers

Si le script nécessite des privilèges root, configurer sudoers avec des commandes exactes :

```
# /etc/sudoers.d/ssh-frontiere
svc-ssh ALL=(root) NOPASSWD: /opt/scripts/backup/run.sh
svc-ssh ALL=(root) NOPASSWD: /opt/scripts/deploy.sh
```

Ne pas utiliser de globbing dans sudoers. Chaque commande doit être listée explicitement.

### 5.6 Timeout et arrêt

Lorsqu'une commande dépasse son timeout :

1. `SIGTERM` est envoyé au process group
2. Attente de 5 secondes pour un arrêt gracieux
3. `SIGKILL` si le processus est toujours vivant

Le timeout est configurable par action (champ `timeout`) ou globalement (`default_timeout`).

---

## 6. Chaînage de commandes

Le protocole v2 supporte le chaînage de commandes dans un même bloc :

### 6.1 Opérateurs

| Opérateur | Syntaxe | Comportement |
|-----------|---------|-------------|
| Séquentiel strict | `;` ou saut de ligne | Arrêt au premier échec |
| Séquentiel permissif | `&` | Continue quoi qu'il arrive |
| Rattrapage | `\|` | Exécute la droite si la gauche échoue |
| Groupement | `( )` | Sous-expression |

### 6.2 Priorité

Du plus faible au plus fort : `;` et `&` (même priorité, gauche à droite) < `|` < `()`.

### 6.3 Exemples

```
# Séquentiel strict (2 commandes, arrêt si la première échoue)
infra healthcheck
infra deploy service=api version=1.2.3
.

# Séquentiel permissif (continue même si la première échoue)
monitoring check & infra deploy service=api version=1.2.3
.

# Rattrapage (deploy si le healthcheck échoue)
infra healthcheck | infra deploy service=api version=1.2.3
.

# Groupement
(infra deploy service=api version=1.2.3 ; infra healthcheck) | infra rollback service=api
.
```

Chaque commande du bloc produit sa propre réponse `>>>`. Le body (`+ body`) est transmis uniquement à la première commande du bloc.

---

## 7. Maintenance

### 7.1 Logs JSON structurés

Chaque commande (autorisée ou refusée) est journalisée en JSON, une entrée par ligne :

```json
{"event":"executed","timestamp":"2026-03-21T14:32:15Z","pid":12345,"domain":"infra","action":"healthcheck","ssh_client":"192.168.1.10 54321 22","effective_tags":[],"action_tags":[]}
```

| Champ | Description |
|-------|-------------|
| `event` | Type d'événement : `executed`, `rejected`, `timeout`, `auth_lockout`, `client_comment`, `stdin_error` |
| `timestamp` | Horodatage ISO 8601 UTC |
| `pid` | PID du processus SSH Frontière |
| `domain` | Domaine de la commande |
| `action` | Action de la commande |
| `reason` | Raison du rejet (si applicable) |
| `ssh_client` | IP et port du client SSH |
| `session_id` | UUID de la session |
| `args` | Arguments de la commande |
| `effective_tags` | Tags effectifs du client |
| `action_tags` | Tags de l'action |
| `defaults_applied` | Arguments dont la valeur par défaut a été utilisée |

### 7.2 Rotation des logs

SSH Frontière écrit en mode append. Utiliser logrotate pour la rotation :

```
# /etc/logrotate.d/ssh-frontiere
/var/log/ssh-frontiere/commands.json {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    copytruncate
}
```

`copytruncate` est recommandé car SSH Frontière n'a pas de mécanisme de rechargement de fichier de log (chaque connexion ouvre le fichier indépendamment).

### 7.3 Mise à jour du binaire

SSH Frontière est sans état : chaque connexion SSH lance un nouveau processus qui lit la configuration depuis le disque. Il n'y a pas de démon à recharger.

Procédure de mise à jour :

```bash
# 1. Sauvegarder l'ancien binaire
cp /usr/local/bin/ssh-frontiere /usr/local/bin/ssh-frontiere.bak

# 2. Copier le nouveau binaire
cp ssh-frontiere /usr/local/bin/ssh-frontiere
chmod 755 /usr/local/bin/ssh-frontiere

# 3. Vérifier
ssh-frontiere --check-config --config=/etc/ssh-frontiere/config.toml
```

Les connexions SSH en cours continuent avec l'ancien binaire. Les nouvelles connexions utilisent le nouveau. Aucune interruption de service.

### 7.4 Rollback

```bash
# Restaurer l'ancien binaire
cp /usr/local/bin/ssh-frontiere.bak /usr/local/bin/ssh-frontiere
```

### 7.5 Mode diagnostic

L'argument `--diagnostic` affiche les erreurs internes détaillées au lieu du message générique `service unavailable` :

```
command="/usr/local/bin/ssh-frontiere --level=ops --diagnostic",restrict ssh-ed25519 AAAA...
```

À utiliser uniquement pour le dépannage, pas en production (fuite d'informations internes).

---

## 8. Dépannage

### 8.1 Erreurs courantes

| Symptôme | Cause probable | Solution |
|----------|---------------|----------|
| `ssh-frontiere: error: service unavailable` | Fichier config introuvable ou invalide | Vérifier le chemin `--config`, lancer `--check-config` |
| Code 128 : `rejected: unknown domain` | Domaine non défini dans `config.toml` | Ajouter le domaine dans la configuration |
| Code 128 : `rejected: unknown action` | Action non définie dans le domaine | Vérifier le nom de l'action |
| Code 128 : `rejected: missing required argument` | Argument obligatoire manquant | Ajouter `clé=valeur` à la commande |
| Code 131 : `rejected: insufficient level` | Niveau RBAC insuffisant | Utiliser un `--level` ou token de niveau supérieur |
| Code 131 : `rejected: access denied (tag mismatch)` | Token sans les tags requis par l'action | Ajouter les tags au token |
| Code 130 : `timeout` | Commande dépassant le délai | Augmenter le `timeout` de l'action |
| Code 132 : `protocol error` | Format de protocole invalide | Vérifier le format des en-têtes et commandes |
| Code 133 : `stdin closed` | Processus enfant a fermé stdin pendant l'écriture du body | Vérifier que le script lit stdin avant de terminer |
| `auth failed (1/3)` | Proof invalide | Vérifier le calcul du proof et le secret |
| `session terminated` | 3 échecs d'auth consécutifs | Vérifier le secret du token et le nonce |

### 8.2 Tester une connexion

```bash
# Test simple (mode one-shot)
echo -e "# test\ninfra healthcheck\n." | ssh svc-ssh@serveur

# Test avec authentification
echo -e "+ auth token=runner-ci proof=$(ssh-frontiere-proof --secret mon-secret)\ninfra deploy service=api version=1.2.3\n." | ssh svc-ssh@serveur

# Mode session interactif (attention : pas de PTY)
ssh svc-ssh@serveur << 'EOF'
+ session keepalive
infra healthcheck
.
infra deploy service=api version=1.2.3
.
.
EOF
```

### 8.3 Vérifier les logs

```bash
# Dernières entrées
tail -5 /var/log/ssh-frontiere/commands.json | jq .

# Commandes rejetées
grep '"event":"rejected"' /var/log/ssh-frontiere/commands.json | jq .

# Verrouillages
grep '"event":"auth_lockout"' /var/log/ssh-frontiere/commands.json | jq .
```

### 8.4 Limites de sécurité

- SSH Frontière ne lance jamais de shell (`/bin/bash`, `/bin/sh`)
- Pas de pipe, redirection ni chaînage shell (`|`, `>`, `&&`, `;` sont des opérateurs du protocole, pas du shell)
- Chaque commande est validée contre la whitelist exacte
- Les arguments sont substitués individuellement (pas d'injection possible)
- L'exécution utilise `execve` directement, sans intermédiaire shell
- L'environnement est vidé : seuls `PATH` et `SSH_FRONTIERE_SESSION` sont transmis

---

## Annexe : exemple complet de configuration

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300
timeout_session = 3600
max_auth_failures = 3
log_comments = true
max_stream_bytes = 10485760

[auth]
challenge_nonce = true

[auth.tokens.runner-ci]
secret = "b64:bW9uLXNlY3JldC1ydW5uZXI="
level = "ops"
tags = ["deploy"]

[auth.tokens.admin-ops]
secret = "b64:bW9uLXNlY3JldC1hZG1pbg=="
level = "admin"
tags = ["deploy", "admin"]

# --- Domaine : infrastructure ---
[domains.infra]
description = "Gestion de l'infrastructure"

[domains.infra.actions.healthcheck]
description = "Vérifier l'état des services"
level = "read"
timeout = 30
execute = "/opt/scripts/infra/healthcheck.sh"

[domains.infra.actions.deploy]
description = "Déployer un service"
level = "ops"
timeout = 600
execute = "/opt/scripts/infra/deploy.sh {service} {version}"
tags = ["deploy"]

[domains.infra.actions.deploy.args]
service = { type = "enum", values = ["api", "web", "worker"] }
version = { type = "string" }

[domains.infra.actions.restart]
description = "Redémarrer un service"
level = "admin"
timeout = 120
execute = "/opt/scripts/infra/restart.sh {service}"
tags = ["admin"]

[domains.infra.actions.restart.args]
service = { type = "enum", values = ["api", "web", "worker", "database"] }

# --- Domaine : backup ---
[domains.backup]
description = "Gestion des sauvegardes"

[domains.backup.actions.run]
description = "Lancer un backup"
level = "ops"
timeout = 3600
execute = "/opt/scripts/backup/run.sh {target}"

[domains.backup.actions.run.args]
target = { type = "enum", values = ["database", "files", "all"], default = "all" }

[domains.backup.actions.status]
description = "Statut du dernier backup"
level = "read"
timeout = 30
execute = "/opt/scripts/backup/status.sh"
```
