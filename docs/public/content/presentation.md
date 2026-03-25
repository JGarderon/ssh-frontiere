+++
title = "Présentation"
description = "Découvrir SSH-Frontière : ce que c'est, pourquoi ça existe, comment ça marche"
date = 2026-03-24
weight = 1
+++

# Présentation de SSH-Frontière

## Le problème

Sur un serveur Linux, les comptes de service SSH (runners CI, agents IA, scripts de maintenance) utilisent généralement `/bin/bash` comme shell de connexion. Cela pose plusieurs problèmes :

- **Aucun contrôle** : le client SSH peut exécuter n'importe quelle commande
- **Pas d'audit** : les commandes exécutées ne sont pas journalisées de manière structurée
- **Pas de granularité** : un script qui a besoin de lire un statut a les mêmes droits qu'un script de déploiement

Les solutions classiques (`authorized_keys` avec `command=`, scripts wrapper bash, bastions SSH) ont chacune leurs limites : fragiles, difficiles à auditer, ou surdimensionnées pour le besoin.

## Ce que fait SSH-Frontière

SSH-Frontière est un **login shell de remplacement**. Il se place entre `sshd` et les commandes du système :

```
Client SSH
    |
    v
sshd (authentification par clé)
    |
    v
ssh-frontiere (login shell)
    |
    ├── Valide la commande contre la configuration TOML
    ├── Vérifie le niveau d'accès (read / ops / admin)
    ├── Exécute la commande autorisée
    └── Renvoie le résultat en JSON structuré
```

Chaque connexion SSH crée un nouveau processus `ssh-frontiere` qui :

1. Affiche une bannière et les capacités du serveur
2. Lit les en-têtes du client (authentification, mode session)
3. Lit la commande (`domaine action [arguments]`, texte brut)
4. Valide contre la whitelist TOML
5. Exécute si autorisé, refuse sinon
6. Renvoie une réponse JSON et se termine

Le programme est **synchrone et éphémère** : pas de daemon, pas de service, pas d'état persistant.

## Ce que SSH-Frontière ne fait pas

- **Pas un bastion SSH** : pas de proxy, pas de relais de connexion vers d'autres serveurs
- **Pas un gestionnaire de clés** : la gestion des clés SSH reste dans `authorized_keys` et `sshd`
- **Pas un shell** : pas d'interprétation de commandes, pas de pipe, pas de redirection, pas d'interactivité
- **Pas un daemon** : s'exécute et meurt à chaque connexion

## Cas d'usage concrets

### Automatisation CI/CD

Un runner Forgejo Actions déploie une application via SSH :

```bash
# Le runner envoie la commande via SSH
{
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@serveur
```

SSH-Frontière vérifie que le runner a le niveau `admin`, que l'action `deploy` existe dans le domaine `forgejo`, que l'argument `version=stable` est une valeur autorisée, puis exécute le script de déploiement configuré.

### Agents IA

Un agent Claude Code agit sur un serveur avec des droits bornés :

```bash
# L'agent découvre les commandes disponibles
{ echo "list"; echo "."; } | ssh agent-ia@serveur

# L'agent exécute une action spécifique
{ echo "infra healthcheck"; echo "."; } | ssh agent-ia@serveur
```

L'agent n'a accès qu'aux actions de niveau `read` configurées pour lui. Les commandes `help` et `list` lui permettent de découvrir les actions disponibles et leurs paramètres — format JSON, nativement parsable.

### Maintenance automatisée

Des scripts cron exécutent des sauvegardes via SSH :

```bash
# Sauvegarde nocturne
{ echo "forgejo backup-config"; echo "."; } | ssh backup@serveur

# Notification après déploiement
{ echo 'notify send message="Déploiement terminé"'; echo "."; } | ssh notify@serveur
```

### Notifications

Déclencher des notifications (Slack, Olvid, email) comme des actions SSH-Frontière standard :

```bash
{ echo 'notify slack channel=ops message="Build OK"'; echo "."; } | ssh notify@serveur
```

## Pourquoi SSH-Frontière plutôt que...

### ...des scripts bash dans `authorized_keys` ?

L'option `command=` dans `authorized_keys` permet de forcer une commande, mais :
- Un seul script par clé — pas de granularité
- Pas de validation des arguments
- Pas de niveaux d'accès
- Pas de logging structuré
- Le script bash peut contenir des vulnérabilités (injection, globbing)

SSH-Frontière offre une configuration déclarative, du RBAC, du logging JSON, et un parseur grammatical qui élimine les injections.

### ...un bastion SSH (Teleport, Boundary) ?

Les bastions SSH sont conçus pour gérer l'accès de **personnes** à des serveurs :
- Lourds à déployer et à maintenir
- Surdimensionnés pour des comptes de service
- Modèle de menace différent (utilisateur interactif vs script automatisé)

SSH-Frontière est un composant léger (~1 Mo) conçu pour les **comptes de service** : pas de session interactive, pas de proxy, juste une validation de commandes.

### ...`sudo` seul ?

`sudo` contrôle l'élévation de privilèges, mais :
- Ne contrôle pas ce que le client SSH peut *demander*
- Pas de protocole structuré (entrées/sorties JSON)
- Pas de logging intégré au niveau de la commande SSH

SSH-Frontière et `sudo` sont complémentaires : SSH-Frontière valide la commande entrante, `sudo` contrôle les privilèges système. C'est la couche 2 et la couche 3 de la défense en profondeur.

## L'intérêt du produit

SSH-Frontière apporte une **gouvernance déclarative** des accès SSH de service :

1. **Tout est dans un fichier TOML** : les domaines, les actions, les arguments, les niveaux d'accès. Pas de logique dispersée dans des scripts.

2. **Déploiement instantané** : comme toute la configuration est centralisée dans un seul fichier TOML, déployer une nouvelle version est trivial. Chaque connexion SSH crée un nouveau processus qui relit la configuration — les changements sont pris en compte dès la fin de la session en cours ou immédiatement pour tout nouveau client.

3. **Zéro confiance par défaut** : rien ne s'exécute sans être explicitement configuré. Pas de shell, pas d'injection possible.

4. **Auditable** : chaque tentative (autorisée ou refusée) est journalisée en JSON structuré avec timestamp, commande, arguments, niveau, résultat.

5. **Compatible LLM** : les agents IA peuvent découvrir les actions disponibles via `help`/`list`, et interagir via un protocole JSON structuré — pas besoin de parser du texte libre.

6. **Européen et open source** : licence EUPL-1.2, développé en France, pas de dépendance à un écosystème propriétaire.

---

Pour aller plus loin : [Installation](@/installation/_index.md) | [Architecture](@/architecture.md) | [Sécurité](@/securite.md) | [Alternatives](@/alternatives.md)
