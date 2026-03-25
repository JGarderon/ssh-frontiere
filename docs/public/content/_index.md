+++
title = "SSH-Frontière"
description = "Login shell SSH restreint en Rust — contrôle déclaratif des connexions entrantes"
sort_by = "weight"

[extra]
framed = true
+++

# SSH-Frontière

**Login shell SSH restreint en Rust** — un point d'entrée unique et sécurisé pour toutes les connexions SSH entrantes.

SSH-Frontière remplace le shell par défaut d'un compte Unix (`/bin/bash`) par un programme qui **valide chaque commande** contre une configuration déclarative en TOML, avant de l'exécuter.

[![GitHub](https://img.shields.io/badge/GitHub-Dépôt_open--source-181717?logo=github&logoColor=white&style=for-the-badge)](https://github.com/JGarderon/ssh-frontiere)

---

## Pourquoi SSH-Frontière ?

**Sécurité par défaut** — Aucune commande ne s'exécute sans être explicitement autorisée. Deny by default, pas de shell, pas d'injection possible.

**Simple à déployer** — Un binaire statique de ~1 Mo, un fichier TOML, une ligne dans `/etc/passwd`. Pas de daemon, pas de service à gérer.

**Flexible** — Trois niveaux d'accès (read, ops, admin), des tags de visibilité, un protocole d'en-têtes structuré. Compatible avec les agents IA, les runners CI/CD, et les scripts de maintenance.

**Auditable** — Chaque commande exécutée ou refusée est journalisée en JSON structuré. 399 tests cargo + 72 scénarios E2E SSH.

---

## Cas d'usage

- **Runners CI/CD** (Forgejo Actions, GitHub Actions) : déploiements, backups, healthchecks via SSH
- **Agents IA** (Claude Code, etc.) : accès contrôlé à des ressources serveur avec niveaux de confiance
- **Maintenance automatisée** : scripts de sauvegarde, de surveillance, de notification

---

## En bref

| | |
|---|---|
| **Langage** | Rust (binaire statique musl, ~1 Mo) |
| **Licence** | [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12) — Licence Publique de l'Union européenne |
| **Tests** | 399 cargo + 72 E2E SSH + 9 harnesses fuzz |
| **Dépendances** | 3 crates directes (`serde`, `serde_json`, `toml`) |
| **Configuration** | TOML déclaratif |
| **Protocole** | En-têtes texte sur stdin/stdout, réponses JSON |

---

## Pour commencer

- [Découvrir SSH-Frontière](@/presentation.md) — ce que c'est, ce que ça fait, pourquoi ça existe
- [Installation](@/installation/_index.md) — compiler, configurer, déployer
- [Guides](@/guides/_index.md) — tutoriels pas-à-pas
- [Sécurité](@/securite.md) — modèle de sécurité et garanties
- [Architecture](@/architecture.md) — conception technique
- [Alternatives](@/alternatives.md) — comparaison avec les autres solutions
- [FAQ](@/faq.md) — questions fréquentes
- [Contribuer](@/contribuer.md) — participer au projet
