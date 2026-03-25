+++
title = "Sécurité"
description = "Modèle de sécurité, garanties et limites de SSH-Frontière"
date = 2026-03-24
weight = 2
+++

# Sécurité

SSH-Frontière est un **composant de sécurité**. Sa raison d'être est de restreindre ce que les connexions SSH entrantes peuvent faire. Cette page documente le modèle de sécurité, ce qui a été mis en place, et ce qui n'est pas garanti.

## Modèle de sécurité

### Principe fondamental : deny by default

Rien ne s'exécute sans être explicitement configuré. Si une commande n'est pas dans la whitelist TOML, elle est refusée. Il n'y a pas de mode permissif, pas de fallback vers un shell.

### Trois couches de défense en profondeur

| Couche | Mécanisme | Protection |
|--------|-----------|------------|
| 1 | `command=` + `restrict` dans `authorized_keys` | Force le niveau d'accès, bloque forwarding/PTY |
| 2 | SSH-Frontière (login shell) | Valide la commande contre la whitelist TOML |
| 3 | `sudo` whitelist dans sudoers | Restreint les commandes système privilégiées |

Même si un attaquant compromet une clé SSH (couche 1), il ne peut exécuter que les commandes autorisées dans la whitelist TOML (couche 2). Même s'il contourne la couche 2, il ne peut élever ses privilèges que pour les commandes autorisées dans sudoers (couche 3).

### Parseur grammatical, pas liste noire

SSH-Frontière **n'est pas un shell**. La sécurité ne repose pas sur un filtrage de caractères (pas de liste noire de `|`, `;`, `&`), mais sur un **parseur grammatical**.

La grammaire attendue est `domaine action [key=value ...]`. Tout ce qui ne respecte pas cette structure est rejeté. Les caractères spéciaux entre guillemets sont du contenu d'argument, pas de la syntaxe — ils sont valides.

`std::process::Command` exécute directement, sans passer par un shell intermédiaire. L'injection de commandes est **structurellement impossible**.

### Déterminisme face aux agents IA

Ce fonctionnement est **déterministe** : une commande donnée produit toujours le même résultat de validation, indépendamment du contexte. C'est une propriété essentielle lorsque l'on travaille avec des agents d'IA, dont la nature est justement l'**indéterminisme** — un modèle peut être biaisé, ou la chaîne de production de l'agent peut être corrompue, visant les shells pour récupérer des informations supplémentaires ou exfiltrer des secrets. Avec SSH-Frontière, un agent compromis ne peut pas contourner la whitelist, ne peut pas injecter de commandes dans un shell, et ne peut pas accéder à des ressources non configurées. C'est **structurellement impossible**.

## Ce qui a été mis en place

### Langage Rust

SSH-Frontière est écrit en Rust, ce qui élimine les classes de vulnérabilités les plus courantes dans les programmes système :
- Pas de buffer overflow
- Pas de use-after-free
- Pas de null pointer dereference
- Pas d'`unsafe` dans le code (interdit par la configuration lint dans `Cargo.toml` : `unsafe_code = "deny"`)

### 399 tests cargo + 72 scénarios E2E SSH

Le projet est couvert par **399 tests cargo** et **72 scénarios E2E SSH** additionnels :

| Type | Nombre | Description |
|------|--------|-------------|
| Tests unitaires | ~340 | Chaque module teste indépendamment (10 fichiers `*_tests.rs`) |
| Tests d'intégration | 50 | Scénarios stdio complets (exécution du binaire) |
| Tests de conformité | 1 (6 scénarios) | Validation du contrat d'interface JSON (ADR 0003) |
| Tests proptest | 8 | Tests de propriétés (fuzzing guidé par contraintes) |
| **Total cargo** | **399** | |
| Scénarios E2E SSH | 72 | Docker Compose avec vrai serveur SSH |
| Harnesses cargo-fuzz | 9 | Fuzzing non-guidé (mutations aléatoires) |

Les tests E2E SSH couvrent le protocole complet, l'authentification, les sessions, la sécurité, la robustesse et le logging. Ils s'exécutent dans un environnement Docker Compose avec un vrai serveur SSH.

### Audit des dépendances

- `cargo deny` en CI : vérifie les licences et les vulnérabilités connues (base RustSec)
- `cargo audit` : audit de sécurité des dépendances
- `cargo clippy` en mode pedantic : 0 warning autorisé
- Seules 3 dépendances directes : `serde`, `serde_json`, `toml` — toutes largement auditées par la communauté Rust

### Contrôle d'accès RBAC

Trois niveaux de confiance hiérarchiques :

| Niveau | Usage | Exemples |
|--------|-------|----------|
| `read` | Consultation seule | healthcheck, status, list |
| `ops` | Opérations courantes | backup, deploy, restart |
| `admin` | Toutes les actions | configuration, données sensibles |

Chaque action a un niveau requis. Chaque connexion SSH a un niveau effectif (via `--level` dans `authorized_keys` ou via authentification par token).

### Tags de visibilité

En complément du RBAC vertical, des **tags** permettent un filtrage horizontal : un token avec le tag `forgejo` ne voit que les actions taguées `forgejo`, même s'il a le niveau `ops`.

### Authentification par token

Deux modes d'authentification :

- **Mode simple** (`challenge_nonce = false`) : challenge-response `SHA-256(secret)` — le client prouve qu'il connaît le secret
- **Mode nonce** (`challenge_nonce = true`) : challenge-response `SHA-256(XOR_encrypt(secret || nonce, secret))` avec le nonce envoyé par le serveur. Le nonce est régénéré après chaque authentification réussie, empêchant le rejeu d'un proof intercepté

### Protections supplémentaires

- **Timeout** par commande avec kill du process group (SIGTERM puis SIGKILL)
- **Lockout** après N tentatives d'authentification échouées (configurable, défaut : 3)
- **Ban IP** optionnel via commande externe configurable
- **Masquage** des arguments sensibles dans les logs (SHA-256)
- **Limite de taille** sur les sorties capturées (stdout, stderr)
- **Nettoyage d'environnement** : `env_clear()` sur les processus enfants, seuls `PATH` et `SSH_FRONTIERE_SESSION` sont injectés

## Ce qui n'est pas garanti

Aucun logiciel n'est parfait. Voici les limites connues et documentées :

### Compteur XOR 8 bits

L'implémentation cryptographique utilise un compteur XOR avec un keystream limité à 8192 bytes. C'est suffisant pour l'usage actuel (proofs SHA-256 de 64 caractères), mais pas conçu pour chiffrer de gros volumes.

### Fuite de longueur dans la comparaison

La comparaison temps-constant peut révéler la longueur des valeurs comparées. En pratique, les proofs SHA-256 font toujours 64 caractères, ce qui rend cette fuite négligeable.

### Rate limiting par connexion

Le compteur de tentatives d'authentification est local à chaque connexion SSH. Un attaquant peut ouvrir N connexions et avoir N x `max_auth_failures` tentatives. Recommandation : coupler avec fail2ban, `sshd MaxAuthTries`, ou des règles iptables.

### Signaler une vulnérabilité

**Ne signalez pas les vulnérabilités via les issues publiques.** Contactez directement le mainteneur pour une divulgation responsable. Le processus est décrit dans le [guide de contribution](@/contribuer.md).

## Dépendances

SSH-Frontière a une politique stricte de dépendances minimales. Chaque crate externe est évaluée selon une matrice pondérée (licence, gouvernance, communauté, taille, dépendances transitives).

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| `serde` | 1.x | Sérialisation/désérialisation | Standard de facto Rust, requis pour JSON et TOML |
| `serde_json` | 1.x | Réponses JSON | Format de sortie du protocole |
| `toml` | 0.8.x | Chargement de la configuration | Format standard Rust pour la configuration |

Dev-dépendance : `proptest` (tests de propriétés uniquement, pas dans le binaire final).

Sources autorisées : **crates.io uniquement**. Aucun dépôt git externe autorisé. Politique vérifiée par `cargo deny`.
