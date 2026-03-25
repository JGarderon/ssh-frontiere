# ADR 0002 — Modèle de secrets à trois niveaux

**Date** : 2026-03-15
**Statut** : Proposée
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : Exercice d'alignement 001, section 7 (Gestion des secrets)
**Voir aussi** : ADR 0001 (format TOML — stockage configuration, section identités et secrets), ADR 0003 (contrat d'interface — ordre domaine→action, format retour 4 champs), ADR 0004 (struct Context — champ `sensitive` dans ArgDef)

---

## Contexte

SSH Frontière est un composant de sécurité qui manipule potentiellement des informations sensibles à plusieurs niveaux :

1. **Transit** : les commandes SSH et leurs arguments transitent dans les logs
2. **Système** : les scripts d'opération accèdent à des secrets d'infrastructure (BDD, API, S3)
3. **Configuration** : le fichier `config.toml` pourrait contenir des données d'authentification (tokens RBAC futurs)

L'exercice d'alignement 001 (section 7, contribution Julien BO) a défini trois niveaux de gestion des secrets. Cette ADR formalise les décisions techniques pour chaque niveau.

---

## Options

### Niveau 1 — Secrets en transit (logs)

**Problème** : une commande SSH peut contenir des informations sensibles. Le logging JSON capture la commande brute. Si les logs sont exfiltrés (Promtail → Loki → réseau), les secrets sont exposés.

*La décision pour ce niveau a été arbitrée directement par Julien (BO) dans l'exercice d'alignement 001 — voir section Décision ci-dessous.*

### Niveau 2 — Secrets système (infrastructure)

| Option | Description |
|--------|-------------|
| A — Hors périmètre | ssh-frontiere ne gère pas les secrets d'infra |
| B — Injection de secrets | ssh-frontiere injecte des secrets dans l'environnement des commandes |

### Niveau 3 — Secrets en configuration

| Option | Description | Sécurité | Lisibilité |
|--------|-------------|----------|------------|
| A — Clair | Valeurs en texte brut dans le TOML | Nulle | Immédiate |
| B — Base64 | Encodage base64, réversible mais pas lisible au survol | Sociale | Facile |
| C — Chiffrement | Chiffré par clé maître | Forte | Nécessite déchiffrement |

---

## Décision

### Niveau 1 — Masquage SHA-256 optionnel, protocole challenge-response pour l'authentification

**Masquage SHA-256 configurable + omission sélective (retour Julien BO, 2026-03-15).**

Pour le MVP (Phase 1), le mécanisme est simple :
- Les commandes et arguments sont loggés normalement (ce sont des identifiants publics, pas des secrets)
- Si un champ est marqué `sensitive` dans la configuration d'un argument, sa valeur **peut** être remplacée par son empreinte SHA-256 dans les logs
- **Le masquage SHA-256 est une OPTION configurable, pas une obligation.** L'administrateur active ou désactive le masquage via `mask_sensitive = true|false` dans `[global]` (cf. ADR 0001). Par défaut : désactivé.

**Justification du caractère optionnel** *(retour Julien BO)* : imposer le masquage systématique est une contrainte disproportionnée pour le MVP. Certains déploiements n'ont pas d'arguments sensibles. L'administrateur choisit son niveau de protection.

#### Protocole d'authentification challenge-response *(Julien, BO)*

Pour l'authentification RBAC future (Phase 3+), le secret ne transite **jamais** en clair sur le réseau. Le protocole est défini par Julien (BO) :

```
1. Client → Serveur : demande de jeton aléatoire (challenge)
2. Serveur → Client : jeton aléatoire T (nonce)
3. Client : calcule empreinte(secret + T), chiffre le résultat avec le secret
4. Client → Serveur : envoie le résultat chiffré
5. Serveur : connaît le secret, recalcule empreinte(secret + T),
             vérifie la correspondance
```

**Propriétés de sécurité** :
- Un attaquant qui intercepte les échanges voit T (public) et le résultat chiffré, mais ne peut pas reconstituer le secret
- Le jeton T est à usage unique (nonce) — un rejeu est impossible
- Le secret n'est jamais transmis, ni en clair ni sous forme d'empreinte réversible

**Implication architecturale** : ce protocole nécessite **une session minimale de 2 échanges** (demande de jeton + envoi de la preuve). En mode ping/pong strict (1 connexion = 1 commande), cela signifie soit :
- **Option A** : 2 connexions SSH successives (connexion 1 = demande de jeton, connexion 2 = commande + preuve). Nécessite un stockage temporaire du jeton côté serveur.
- **Option B** : 1 connexion SSH avec un mini-protocole sur stdin/stdout (envoi du challenge, attente de la réponse, puis exécution). Cela introduit un mode semi-session.

**Décision** : le choix entre les options A et B est différé à la Phase 3 (ADR dédiée). Le format de configuration réserve la section `[auth]` pour les empreintes de tokens. Pour le MVP, l'authentification repose uniquement sur SSH (`authorized_keys` + `--level`).

**Implémentation MVP** : le champ `sensitive` dans la définition d'argument est **prévu dans le format de configuration** mais le traitement SHA-256 dans le logging peut être implémenté dès la Phase 1 (le coût est faible).

**Note sur SHA-256 sans dépendance externe** : SHA-256 peut être implémenté en ~150 lignes de Rust pur (algorithme public, pas de propriété intellectuelle). Alternativement, si une implémentation auditée est préférable, la crate `sha2` sera évaluée. Pour le MVP, une implémentation maison documentée et testée est acceptable car :
- L'algorithme est standardisé (FIPS 180-4)
- L'usage est non-cryptographique (empreinte de traçabilité, pas de preuve de sécurité)
- Zéro dépendance supplémentaire

### Niveau 2 — Hors périmètre (option A)

**ssh-frontiere est un relai + RBAC, pas un gestionnaire de secrets.**

- Les scripts d'opération (`backup-db.sh`, etc.) accèdent aux secrets par les mécanismes d'administration habituels
- `env_clear()` garantit que ssh-frontiere ne transmet pas de variables d'environnement sensibles aux processus enfants
- Les secrets d'infrastructure (tokens S3, mots de passe BDD) sont gérés par Docker secrets, fichiers protégés, ou variables d'environnement des conteneurs — hors du chemin SSH

### Niveau 3 — Base64 + permissions Unix (option B)

**Les valeurs sensibles dans `config.toml` sont encodées en base64.**

- Ce n'est pas du chiffrement (base64 est réversible) — c'est une protection **sociale** : un regard furtif sur le fichier ne révèle pas les secrets
- La protection principale reste les permissions Unix (`root:root 640`)
- Le format TOML distingue les champs encodés par un préfixe conventionnel : `"b64:dG9rZW4tc2VjcmV0"` ou un type dédié dans le schéma

**Implémentation MVP** : le décodage base64 est prévu dans le format mais peut être différé. Le MVP n'a pas de secrets en configuration (pas de tokens RBAC). Le format est réservé pour la Phase 3+.

---

## Format dans la configuration

```toml
# Niveau 3 — Exemple futur (Phase 3+)
[auth.tokens]
# Token encodé en base64, empreinte SHA-256 pour vérification
runner-forge = { hash = "b64:YTJmNDg2...", level = "ops" }

# Niveau 1 — Argument sensible (MVP)
[domains.forgejo.actions.api-call]
description = "Appel API authentifié"
level = "ops"
timeout = 30
execute = "sudo /usr/local/bin/api-call.sh {token}"
[[domains.forgejo.actions.api-call.args]]
name = "token"
type = "string"
sensitive = true   # ← SHA-256 dans les logs
```

---

## Conséquences

### Positives

- Séparation claire des responsabilités (3 niveaux distincts)
- Masquage SHA-256 optionnel — pas de contrainte disproportionnée
- Protocole challenge-response documenté pour l'authentification future — le secret ne transite jamais
- Pas de secret en clair visible dans la configuration (base64)
- ssh-frontiere ne devient pas un gestionnaire de secrets (niveau 2 hors périmètre)
- SHA-256 implémentable sans dépendance externe

### Négatives

- Base64 n'est pas du chiffrement — un attaquant avec accès lecture au fichier peut décoder
- SHA-256 maison nécessite des tests rigoureux (vecteurs de test NIST)
- Le champ `sensitive` ajoute de la complexité au format de configuration

### Risques

- Si l'implémentation SHA-256 maison a un bug, les empreintes seront incorrectes (mais pas dangereuses — le pire cas est un log inutile, pas une fuite de secret)
- L'encodage base64 peut donner un faux sentiment de sécurité — la documentation doit être explicite sur sa nature

### Ce qui est implémenté au MVP

| Élément | Phase 1 (MVP) | Phase 3+ |
|---------|---------------|----------|
| Champ `sensitive` dans args | Format prévu, flag parsé, masquage opérationnel | Inchangé |
| `mask_sensitive` dans `[global]` | Implémenté (défaut: false) | Inchangé |
| SHA-256 dans les logs | Implémenté, activé si `mask_sensitive = true` | Recommandé si args sensibles |
| Protocole challenge-response | Non implémenté, format `[auth]` réservé | Session minimale 2 échanges, ADR dédiée |
| Base64 en config | Format réservé, pas de décodage | Décodage pour tokens auth |
| env_clear() | Oui (sécurité de base) | Oui |

---

## Attribution

- **Julien (BO)** : modèle à 3 niveaux, SHA-256 optionnel (pas imposé), protocole challenge-response pour l'authentification, base64 en config, hors périmètre pour les secrets système
- **Claude (PM/Tech Lead)** : analyse des options, phasage MVP vs futur, implémentation SHA-256 sans dépendance, formalisation du protocole challenge-response
- **Agents Claude Code** : implémentation, tests vecteurs NIST
