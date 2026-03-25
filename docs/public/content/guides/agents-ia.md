+++
title = "Agents IA"
description = "Utiliser SSH-Frontière avec des agents IA (Claude Code, etc.)"
date = 2026-03-24
weight = 4
+++

# Utiliser SSH-Frontière avec des agents IA

SSH-Frontière a été conçu dès l'origine pour être compatible avec les agents IA (LLM). Le protocole structuré, la découverte automatique et les réponses JSON en font un point d'entrée idéal pour les agents qui ont besoin d'agir sur un serveur.

## Pourquoi SSH-Frontière pour les agents IA ?

Les agents IA (Claude Code, Cursor, GPT, etc.) peuvent exécuter des commandes sur un serveur via SSH. Le problème : sans contrôle, un agent peut exécuter n'importe quoi.

SSH-Frontière résout ce problème :

- **Borner les actions** : l'agent ne peut exécuter que les commandes configurées
- **Niveaux d'accès** : un agent en `read` ne peut que consulter, pas modifier
- **Découverte** : l'agent peut demander `help` pour connaître les actions disponibles
- **JSON structuré** : les réponses sont directement parsables par l'agent

## Configuration pour un agent IA

### 1. Clé SSH dédiée

Générez une clé SSH pour l'agent :

```bash
ssh-keygen -t ed25519 -C "agent-claude" -f ~/.ssh/agent-claude
```

### 2. Niveau de confiance restreint

Dans `authorized_keys`, donnez un niveau minimal :

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... agent-claude
```

Commencez par `read` et élevez si nécessaire via un token.

### 3. Domaines dédiés

Configurez des actions spécifiques pour l'agent :

```toml
[domains.agent]
description = "Actions pour agents IA"

[domains.agent.actions.status]
description = "État des services"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-all.sh"

[domains.agent.actions.logs]
description = "Derniers logs applicatifs"
level = "read"
timeout = 30
execute = "/usr/local/bin/recent-logs.sh {service}"

[domains.agent.actions.logs.args]
service = { type = "enum", values = ["web", "api", "worker", "database"] }

[domains.agent.actions.restart]
description = "Redémarrer un service"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-service.sh {service}"
tags = ["agent-ops"]

[domains.agent.actions.restart.args]
service = { type = "enum", values = ["web", "api", "worker"] }
```

### 4. Token pour élévation (optionnel)

Si l'agent a besoin d'accéder à des actions `ops` :

```toml
[auth.tokens.agent-claude]
secret = "b64:c2VjcmV0LWFnZW50LWNsYXVkZQ=="
level = "ops"
tags = ["agent-ops"]
```

## Exemple avec Claude Code (AutoClaude)

Un agent Claude Code dans un conteneur AutoClaude peut utiliser SSH-Frontière pour agir sur le serveur hôte :

```bash
# L'agent découvre les commandes disponibles (JSON via list)
{ echo "list"; echo "."; } | ssh -i /keys/agent-claude agent@serveur

# L'agent vérifie l'état des services
{ echo "agent status"; echo "."; } | ssh -i /keys/agent-claude agent@serveur

# L'agent lit les logs d'un service
{ echo "agent logs service=api"; echo "."; } | ssh -i /keys/agent-claude agent@serveur
```

La sortie est envoyée en streaming (`>>`), puis la réponse JSON finale (`>>>`) :

```
>> web: running
>> api: running
>> worker: stopped
>>> {"command":"agent status","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

L'agent peut analyser les lignes `>>` (sortie standard en streaming), détecter que `worker` est arrêté, et décider d'agir en conséquence. La réponse `>>>` confirme le code de retour.

## Mode session

Pour éviter d'ouvrir une connexion SSH par commande, l'agent peut utiliser le mode session :

```bash
{
  echo "+ auth token=agent-claude proof=..."
  echo "+ session keepalive"
  echo "agent status"
  echo "."
  echo "agent logs service=worker"
  echo "."
  echo "."   # bloc vide = fin de session
} | ssh -i /keys/agent-claude agent@serveur
```

Chaque commande est suivie de `.` (fin de bloc). Un `.` sans commande précédente signale la fin de session. Le mode session permet d'envoyer plusieurs commandes dans une seule connexion SSH, avec un timeout global configurable (`timeout_session`).

## Bonnes pratiques

1. **Principe du moindre privilège** : commencez par `read`, élevez par token uniquement si nécessaire
2. **Actions atomiques** : chaque action fait une seule chose. L'agent compose les actions entre elles
3. **Noms explicites** : les noms de domaines et actions sont visibles par `help` — rendez-les compréhensibles
4. **Tags de visibilité** : isolez les actions de l'agent avec des tags dédiés
5. **Limites de sortie** : configurez `max_stdout_chars` pour éviter que l'agent ne reçoive des volumes trop importants
6. **Logs** : surveillez les logs pour détecter les usages anormaux

---

**Suite** : [Intégration CI/CD](@/guides/ci-cd.md) — automatiser les déploiements via SSH-Frontière.
