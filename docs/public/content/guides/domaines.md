+++
title = "Domaines et actions"
description = "Configurer des domaines et des actions dans SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Configurer des domaines et actions

Un **domaine** est un périmètre fonctionnel (une application, un service, une catégorie d'opérations). Chaque domaine contient des **actions** : les commandes autorisées.

## Ajouter un domaine de déploiement

```toml
[domains.monapp]
description = "Application web principale"

[domains.monapp.actions.deploy]
description = "Déployer une version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy-monapp.sh {tag}"

[domains.monapp.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.monapp.actions.status]
description = "Vérifier l'état du service"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-monapp.sh"

[domains.monapp.actions.restart]
description = "Redémarrer le service"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-monapp.sh"
```

Utilisation :

```bash
# Déployer la version stable
{ echo "monapp deploy version=stable"; echo "."; } | ssh ops@serveur

# Vérifier l'état
{ echo "monapp status"; echo "."; } | ssh monitoring@serveur

# Redémarrer
{ echo "monapp restart"; echo "."; } | ssh ops@serveur
```

## Ajouter un domaine de sauvegarde

```toml
[domains.backup]
description = "Sauvegardes automatisées"

[domains.backup.actions.full]
description = "Sauvegarde complète"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-full.sh {domain}"

[domains.backup.actions.config-only]
description = "Sauvegarde de la configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
```

## Ajouter un domaine de notification

```toml
[domains.notify]
description = "Notifications"

[domains.notify.actions.slack]
description = "Envoyer une notification Slack"
level = "ops"
timeout = 30
execute = "/usr/local/bin/notify-slack.sh {channel} {message}"

[domains.notify.actions.slack.args]
channel = { type = "enum", values = ["general", "ops", "alerts"], default = "ops" }
message = { free = true }
```

L'argument `message` est déclaré avec `free = true` : il accepte n'importe quelle valeur textuelle.

```bash
{ echo 'notify slack channel=ops message="Déploiement terminé"'; echo "."; } | ssh ops@serveur
```

## Ajouter un domaine de maintenance

```toml
[domains.infra]
description = "Infrastructure serveur"

[domains.infra.actions.healthcheck]
description = "Vérification de santé des services"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[domains.infra.actions.disk-usage]
description = "Espace disque"
level = "read"
timeout = 10
execute = "/usr/bin/df -h"

[domains.infra.actions.logs]
description = "Derniers logs système"
level = "ops"
timeout = 30
execute = "sudo /usr/bin/journalctl -n 100 --no-pager"
```

## Checklist après ajout d'une action

1. Vérifier la syntaxe TOML (une erreur = fail-fast, code 129)
2. Créer le script d'exécution si nécessaire
3. Ajouter dans sudoers si la commande utilise `sudo`
4. Tester avec `ssh user@serveur` depuis un autre terminal
5. Vérifier les logs dans `/var/log/ssh-frontiere/commands.json`

## Découverte

Les commandes `help` et `list` permettent de voir les actions disponibles :

```bash
# Liste complète avec descriptions (texte lisible via #>)
{ echo "help"; echo "."; } | ssh user@serveur

# Détails d'un domaine (texte lisible via #>)
{ echo "help monapp"; echo "."; } | ssh user@serveur

# Liste courte en JSON (domaine + action)
{ echo "list"; echo "."; } | ssh user@serveur
```

`help` renvoie du texte lisible (préfixe `#>`). `list` renvoie du JSON structuré — plus adapté au parsing automatique. Les deux ne montrent que les actions accessibles au niveau effectif du client.

---

**Suite** : [Tokens et niveaux de sécurité](@/guides/tokens.md) — contrôler qui peut faire quoi.
