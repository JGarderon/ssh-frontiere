+++
title = "Alternatives"
description = "Comparaison de SSH-Frontière avec les solutions existantes de contrôle SSH"
date = 2026-03-24
weight = 4
+++

# Comparaison avec les alternatives

SSH-Frontière n'est pas la seule façon de contrôler les accès SSH. Cette page compare les approches existantes pour vous aider à choisir la bonne solution.

## Tableau comparatif

| Critère | `authorized_keys` `command=` | SSH-Frontière | Teleport | Boundary |
|---------|------------------------------|---------------|----------|----------|
| **Type** | Option OpenSSH | Login shell | Bastion SSH | Bastion SSH |
| **Cible** | Script unique par clé | Comptes de service | Utilisateurs humains | Utilisateurs humains |
| **Granularité** | 1 commande par clé | RBAC 3 niveaux, domaines, actions, arguments | Rôles, labels, RBAC | Politiques IAM |
| **Logging** | Non structuré | JSON structuré par commande | Session complète (replay) | Audit trail |
| **Déploiement** | Natif (OpenSSH) | 1 binaire + 1 fichier TOML | Cluster (auth server, proxy, node) | Cluster (controller, workers) |
| **Dépendances** | Aucune | 0 dépendance système | Base de données, certificats | Base de données |
| **Taille** | — | ~1 Mo (binaire statique) | ~100 Mo | ~100 Mo |
| **Anti-injection** | Responsabilité du script | Structurel (parseur grammatical) | N/A (session interactive) | N/A (session interactive) |
| **Compatible LLM** | Non | Oui (JSON, help, découverte) | Non | Non |
| **Licence** | OpenSSH (BSD) | EUPL-1.2 | AGPL-3.0 (OSS) / Commercial | BSL 1.1 |

## `authorized_keys` avec `command=`

L'option `command=` dans `authorized_keys` permet de forcer l'exécution d'un script à chaque connexion. C'est la solution la plus simple et la plus répandue.

### Avantages

- **Zéro installation** : fonctionnalité native d'OpenSSH
- **Simple** pour un cas d'usage unique (une clé = une commande)

### Limites

- **Un seul script par clé** : pas de granularité fine. Pour N actions différentes, il faut N clés ou un script bash qui parse `$SSH_ORIGINAL_COMMAND`
- **Pas de validation des arguments** : le script reçoit une chaîne brute et doit la valider lui-même — source d'injection si mal fait
- **Pas de niveaux d'accès** : toutes les clés ont les mêmes droits (ou il faut les coder dans le script)
- **Pas de logging structuré** : les logs dépendent du script
- **Fragile** : un script bash avec validation de commandes est difficile à sécuriser et à maintenir

### Quand choisir `command=`

- Besoin simple : une clé SSH, une commande fixe, pas de paramètres
- Pas d'exigence d'audit ou de RBAC

## Teleport

[Teleport](https://goteleport.com/) est un bastion SSH complet avec enregistrement de sessions, SSO, certificats, et audit trail.

### Avantages

- **Enregistrement de session** : replay complet de chaque session SSH
- **SSO intégré** : GitHub, OIDC, SAML
- **Certificats** : pas de gestion de clés SSH
- **Audit complet** : qui s'est connecté, quand, depuis où, ce qui a été fait

### Limites

- **Complexe à déployer** : auth server, proxy, node agent, base de données, certificats
- **Conçu pour les humains** : sessions interactives, pas de protocole machine-to-machine
- **Surdimensionné** pour les comptes de service : un runner CI n'a pas besoin d'enregistrement de session ni de SSO
- **Licence duale** : la version communautaire (AGPL-3.0) a des limites fonctionnelles

### Quand choisir Teleport

- Gestion d'accès de **personnes** à un parc de serveurs
- Besoin d'enregistrement de session et de SSO
- Infrastructure avec des moyens pour déployer et maintenir un cluster

## HashiCorp Boundary

[Boundary](https://www.boundaryproject.io/) est un proxy d'accès qui abstrait les détails de connexion et intègre des sources d'identité externes.

### Avantages

- **Abstraction d'infrastructure** : les utilisateurs se connectent à des cibles logiques, pas à des IP
- **Intégration IAM** : Active Directory, OIDC, LDAP
- **Injection de credentials** : les secrets sont injectés dynamiquement, jamais partagés

### Limites

- **Complexe** : controller, workers, base de données, intégration IAM
- **Orienté utilisateurs humains** : pas conçu pour les scripts automatisés
- **Licence BSL 1.1** : restrictions commerciales sur l'édition communautaire
- **Pas de contrôle au niveau commande** : Boundary contrôle l'accès à un hôte, pas à une commande spécifique

### Quand choisir Boundary

- Grand parc de serveurs avec gestion d'identité centralisée
- Besoin d'abstraction d'infrastructure (les utilisateurs ne connaissent pas les IP)
- Équipe avec expertise HashiCorp (Vault, Terraform, etc.)

## `sudo` seul

`sudo` contrôle l'élévation de privilèges pour les commandes système. Souvent utilisé seul pour restreindre les actions d'un compte de service.

### Avantages

- **Natif** : présent sur tous les systèmes Linux
- **Granulaire** : règles fines par utilisateur, commande et arguments

### Limites

- **Ne contrôle pas l'entrée SSH** : n'importe quelle commande peut être **demandée** via SSH, même si `sudo` bloque l'élévation
- **Pas de protocole** : pas de réponse structurée, pas de logging JSON intégré
- **Configuration complexe** : les règles sudoers deviennent difficiles à maintenir avec de nombreuses commandes

### Quand choisir `sudo` seul

- Environnement simple où le risque est faible
- L'entrée SSH est déjà contrôlée par un autre mécanisme (bastion, VPN)

## Quand choisir SSH-Frontière

SSH-Frontière est conçu pour un **cas d'usage précis** : contrôler ce que les comptes de service (pas les humains) peuvent faire via SSH.

Choisissez SSH-Frontière si :

- Vos connexions SSH sont des **scripts automatisés** (CI/CD, agents IA, cron)
- Vous avez besoin de **granularité** : domaines, actions, arguments, niveaux d'accès
- Vous voulez du **logging JSON structuré** pour l'audit et l'observabilité
- Vous voulez un **déploiement simple** : un binaire, un fichier TOML
- Vous avez besoin de **compatibilité LLM** : réponses JSON, découverte via `help`/`list`
- Vous ne voulez pas déployer et maintenir un cluster (Teleport, Boundary)

Ne choisissez pas SSH-Frontière si :

- Vos utilisateurs sont des **humains** qui ont besoin de sessions interactives riches et complètes
- Vous avez besoin d'un **proxy SSH** vers d'autres serveurs
- Vous avez besoin de **SSO**
