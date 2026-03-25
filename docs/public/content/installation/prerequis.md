+++
title = "Prérequis"
description = "Ce qu'il faut pour installer SSH-Frontière"
date = 2026-03-24
weight = 1
+++

# Prérequis

## Serveur cible

| Élément | Détail |
|---------|--------|
| Système | Linux x86_64 |
| Accès SSH | `sshd` fonctionnel |
| Compte de service | Un utilisateur dédié (ex : `forge-runner`) |
| Compte admin secours | Un compte avec `/bin/bash` (ne sera jamais modifié) |
| Accès console | IPMI, KVM ou console cloud — en cas de lockout SSH |

**Important** : gardez toujours un accès console fonctionnel et un compte admin avec un shell classique. Si le login shell SSH-Frontière est mal configuré, vous pourriez perdre l'accès SSH au compte de service.

## Machine de build

Pour compiler SSH-Frontière depuis les sources :

| Élément | Détail |
|---------|--------|
| Rust | Version 1.70 ou supérieure |
| Cible musl | `x86_64-unknown-linux-musl` (pour un binaire statique) |
| `make` | Optionnel, pour les raccourcis du Makefile |

### Installer la cible musl

```bash
rustup target add x86_64-unknown-linux-musl
```

## Alternative : binaire pré-compilé

Si vous ne souhaitez pas compiler, vous pouvez télécharger le binaire statique depuis les [releases du projet](https://github.com/nothus-forge/ssh-frontiere/releases). Le binaire n'a aucune dépendance système.

---

**Suite** : [Compilation depuis les sources](@/installation/compilation.md)
