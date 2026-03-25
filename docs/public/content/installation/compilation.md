+++
title = "Compilation"
description = "Compiler SSH-Frontière depuis les sources"
date = 2026-03-24
weight = 2
+++

# Compilation depuis les sources

## Compilation release

```bash
# Via make (recommandé)
make release

# Ou directement avec cargo
cargo build --release --target x86_64-unknown-linux-musl
```

Le binaire résultant se trouve dans :

```
target/x86_64-unknown-linux-musl/release/ssh-frontiere
```

C'est un **binaire statique** d'environ 1 Mo, sans aucune dépendance système. Il peut être copié directement sur n'importe quel serveur Linux x86_64.

## Vérification

```bash
# Vérifier le type du binaire
file target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ELF 64-bit LSB executable, x86-64, statically linked

# Vérifier la taille
ls -lh target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ~1-2 Mo
```

## Compilation debug

Pour le développement :

```bash
make build
# ou
cargo build
```

## Tests

Avant de déployer, vérifiez que les tests passent :

```bash
# Tests unitaires et d'intégration
make test

# Lints (formatage + clippy)
make lint

# Audit des dépendances
make audit
```

## Binaire auxiliaire : proof

Un binaire auxiliaire est inclus pour calculer les proofs d'authentification :

```bash
cargo build --release --target x86_64-unknown-linux-musl --bin proof
```

Ce binaire est utile pour tester l'authentification challenge-response sans implémenter le calcul SHA-256 côté client.

---

**Suite** : [Configuration](@/installation/configuration.md) — préparer le fichier `config.toml`.
