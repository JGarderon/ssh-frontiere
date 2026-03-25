+++
title = "Premier usage"
description = "Installer SSH-Frontière, configurer un premier domaine, et tester"
date = 2026-03-24
weight = 1
+++

# Premier usage

Ce guide vous accompagne de l'installation à votre première commande SSH via SSH-Frontière.

## 1. Préparer une configuration minimale

Créez un fichier `config.toml` minimal :

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 60

[domains.test]
description = "Domaine de test"

[domains.test.actions.hello]
description = "Commande de test qui affiche un message"
level = "read"
timeout = 10
execute = "/usr/bin/echo hello from ssh-frontiere"
```

Cette configuration définit un seul domaine `test` avec une action `hello` accessible au niveau `read`.

## 2. Installer et configurer

Vous devez d'abord disposer du binaire `ssh-frontiere`. Voir le [guide de compilation](@/installation/compilation.md) ou téléchargez un binaire pré-compilé depuis la [page des releases](https://github.com/nothus-forge/ssh-frontiere/releases).

```bash
# Copier le binaire
sudo cp ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere

# Installer la configuration
sudo mkdir -p /etc/ssh-frontiere
sudo cp config.toml /etc/ssh-frontiere/config.toml
sudo chmod 640 /etc/ssh-frontiere/config.toml

# Créer le répertoire de logs
sudo mkdir -p /var/log/ssh-frontiere

# Créer le compte de service
sudo useradd -m -s /usr/local/bin/ssh-frontiere test-user

# Donner au compte l'accès en écriture aux logs
sudo chown test-user:test-user /var/log/ssh-frontiere
```

## 3. Configurer la clé SSH

Sur votre machine cliente :

```bash
# Générer une clé
ssh-keygen -t ed25519 -C "test-key" -f ~/.ssh/test-frontiere
```

Sur le serveur, ajoutez la clé publique dans `~test-user/.ssh/authorized_keys` :

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... test-key
```

```bash
# Sécuriser les permissions
sudo chmod 700 ~test-user/.ssh
sudo chmod 600 ~test-user/.ssh/authorized_keys
sudo chown -R test-user:test-user ~test-user/.ssh
```

## 4. Premier appel

```bash
# Découvrir les commandes disponibles
{ echo "help"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

Réponse attendue (le serveur envoie d'abord la bannière, puis la réponse) :

```
#> ssh-frontiere 0.1.0
+> capabilities session, help, body
#> type "help" for available commands
#> ...
>>> {"command":"help","status_code":0,"status_message":"ok","stdout":null,"stderr":null}
```

Les lignes `#>` contiennent le texte d'aide lisible. La commande `help` affiche la liste des domaines et actions accessibles au niveau `read`.

## 5. Exécuter une commande

```bash
{ echo "test hello"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

Réponse attendue :

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

La sortie du programme (`hello from ssh-frontiere`) est envoyée en streaming via `>>`, puis la réponse JSON finale via `>>>`. Les champs `stdout` et `stderr` sont `null` dans le JSON car la sortie a été envoyée en streaming.

## 6. Comprendre le flux

Voici ce qui s'est passé :

1. Le client SSH se connecte avec la clé `test-frontiere`
2. `sshd` authentifie la clé et lit `authorized_keys`
3. L'option `command=` force l'exécution de `ssh-frontiere --level=read`
4. SSH-Frontière affiche la bannière (`#>`, `+>`) et attend les en-têtes
5. Le client envoie la commande `test hello` (texte brut, sans préfixe) puis `.` (fin de bloc)
6. SSH-Frontière valide : domaine `test`, action `hello`, niveau `read` <= `read` requis
7. SSH-Frontière exécute `/usr/bin/echo hello from ssh-frontiere`
8. La sortie est envoyée en streaming (`>>`), puis la réponse JSON finale (`>>>`)

## 7. Tester un rejet

Essayez une commande qui n'existe pas :

```bash
{ echo "test inexistant"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

Réponse :

```
>>> {"command":"test inexistant","status_code":128,"status_message":"rejected: unknown action 'inexistant' in domain 'test'","stdout":null,"stderr":null}
```

`stdout` et `stderr` sont `null` car la commande n'a pas été exécutée.

## Prochaine étape

Maintenant que SSH-Frontière fonctionne, vous pouvez [configurer vos propres domaines et actions](@/guides/domaines.md).
