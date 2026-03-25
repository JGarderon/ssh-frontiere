+++
title = "FAQ"
description = "Questions fréquentes sur SSH-Frontière"
date = 2026-03-24
weight = 5
+++

# Questions fréquentes

## Qu'est-ce que SSH-Frontière exactement ?

Un **login shell de remplacement** écrit en Rust. Il s'installe à la place de `/bin/bash` dans `/etc/passwd` pour un compte de service. Chaque connexion SSH passe par SSH-Frontière qui valide la commande contre un fichier de configuration TOML avant de l'exécuter.

## Est-ce un bastion SSH ?

Non. Un bastion SSH (Teleport, Boundary) est un **proxy** qui relaie les connexions vers d'autres serveurs. SSH-Frontière ne fait pas de relais — il contrôle ce qui s'exécute **sur le serveur où il est installé**.

Les bastions gèrent l'accès de personnes à un parc de serveurs. SSH-Frontière gère l'accès de **comptes de service** (runners CI, agents IA, scripts) à des actions spécifiques sur un serveur.

## Est-ce que ça remplace `sudo` ?

Non, c'est complémentaire. SSH-Frontière contrôle ce que le client SSH **peut demander** (couche 2). `sudo` contrôle les privilèges système **nécessaires à l'exécution** (couche 3). Les deux se combinent pour une défense en profondeur.

## Peut-on l'utiliser sans fichier TOML ?

Non. Le fichier de configuration est obligatoire. C'est voulu : tout est explicite, déclaratif, et auditable. Pas de mode permissif, pas de fallback vers un shell.

## Que se passe-t-il si la configuration est invalide ?

SSH-Frontière valide intégralement la configuration au démarrage (fail-fast). Si la configuration est invalide, le programme s'arrête avec le code 129 et un message d'erreur explicite dans le journal. Aucune commande n'est exécutée. Le client SSH, lui, ne voit **jamais** le détail de l'erreur — seulement que le service n'est pas disponible. Les informations de diagnostic restent côté serveur.

Vous pouvez tester la configuration sans risque :

```bash
ssh-frontiere --check-config --config /etc/ssh-frontiere/config.toml
```

## Comment diagnostiquer un problème ?

Plusieurs outils sont disponibles :

1. **Validation de config** : `ssh-frontiere --check-config` vérifie la syntaxe et la cohérence
2. **Commande `help`** : affiche les actions accessibles au niveau effectif du client
3. **Commande `list`** : version courte (domaine + action)
4. **Logs JSON** : chaque commande (exécutée ou refusée) est journalisée avec timestamp, commande, arguments, niveau, résultat
5. **Code de sortie** : 0 = succès, 128 = refusé, 129 = erreur config, 130 = timeout, 131 = niveau insuffisant, 132 = erreur protocole, 133 = body stdin fermé prématurément

## Les agents IA peuvent-ils l'utiliser ?

Oui, c'est un cas d'usage de première classe. Les commandes `help` et `list` renvoient du JSON structuré, directement parsable par un agent. Le protocole d'en-têtes (préfixes `+`, `#`, `$`, `>`) est conçu pour être lisible par des machines sans perturber la lecture humaine.

Voir le [guide agents IA](@/guides/agents-ia.md) pour la configuration détaillée.

## Quelles sont les dépendances dans le code source ?

3 dépendances directes :

| Crate | Usage |
|-------|-------|
| `serde` + `serde_json` | Sérialisation JSON (logs, réponses) |
| `toml` | Chargement de la configuration |

Pas de runtime async, pas de Tokio, pas de framework web. Le binaire statique fait ~1 Mo.

## Pourquoi Rust et pas Go/Python ?

1. **Sécurité mémoire** : pas de buffer overflow, pas de use-after-free — critique pour un composant de sécurité
2. **Binaire statique** : compile avec musl, aucune dépendance système
3. **Performance** : démarrage en millisecondes, pas de runtime
4. **Pas d'`unsafe`** : interdit par les lints Cargo (`unsafe_code = "deny"`)

## Pourquoi TOML et pas YAML ou JSON ?

- **TOML** : lisible, typé, commentaires, standard Rust, pas d'indentation significative
- **YAML** : indentation significative source d'erreurs, types implicites dangereux (`on`/`off` → booléen)
- **JSON** : pas de commentaires, verbeux, pas conçu pour la configuration humaine

Le choix est documenté dans l'ADR 0001.

## Comment fonctionne l'authentification par token ?

Deux modes :

1. **Mode simple** (`challenge_nonce = false`) : le client calcule `SHA-256(secret)` et l'envoie comme proof
2. **Mode nonce** (`challenge_nonce = true`) : le serveur envoie un nonce, le client calcule `SHA-256(XOR_encrypt(secret || nonce, secret))`

Le mode nonce protège contre le rejeu : chaque proof est unique grâce au nonce.

## Peut-on utiliser plusieurs clés SSH ?

Oui. Chaque clé dans `authorized_keys` a son propre `--level`. Plusieurs clés peuvent coexister avec des niveaux différents :

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin
```

## Quel est le format des réponses ?

La sortie standard et d'erreur sont envoyées en streaming (préfixes `>>` et `>>!`), puis une réponse JSON finale sur une seule ligne (préfixe `>>>`) :

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` dans le JSON final : la sortie a été envoyée en streaming
- `status_code` = 0 : succès (code de sortie du processus enfant en passthrough)

## Comment mettre à jour SSH-Frontière ?

1. Compiler la nouvelle version (`make release`)
2. Copier le binaire sur le serveur (`scp`)
3. Vérifier (`ssh user@serveur` + `help`)

Pas de migration de données, pas de schéma de base de données. Le fichier TOML est versionnable avec git.

## Comment contribuer ?

Voir le [guide de contribution](@/contribuer.md). En résumé : ouvrir une issue, fork, TDD, pull request, CI verte. Les contributions générées par IA sont acceptées.

## Où trouver le code source ?

Le code source est disponible sur le [dépôt GitHub](https://github.com/nothus-forge/ssh-frontiere). Licence [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).
