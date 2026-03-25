# ADR 0010 — Nonce optionnel dans l'authentification

**Date** : 2026-03-18
**Statut** : Accepted (validée par Julien BO, 2026-03-18)
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : ADR 0006 (protocole d'entêtes et auth RBAC — challenge-response §3), ADR 0005 (SHA-256 FIPS 180-4)
**Voir aussi** : ADR 0008 (tags), ADR 0009 (arguments nommés)

---

## Contexte

Le protocole d'authentification actuel (ADR 0006 §3) repose sur un **challenge-response avec nonce obligatoire** :

```
Serveur → client : +> challenge nonce=<hex_32>
Client → serveur : + auth token=<id> proof=<hex_64>
```

Où `proof = SHA-256(XOR_encrypt(secret || nonce, secret))`.

Ce mécanisme protège contre le **replay** : chaque connexion génère un nonce unique, donc une preuve valide pour une connexion est invalide pour la suivante.

### Le problème

L'intégration avec les **runners Forgejo Actions** est bloquée. Dans un pipeline CI/CD, l'appel SSH se fait typiquement en bash via un pipe :

```bash
echo -e "+ auth token=runner proof=$PROOF\ncommande arg=val\n." | ssh user@host
```

Le problème est que le nonce est envoyé par le serveur **dans la bannière**, après l'établissement de la connexion. Pour calculer la preuve, le client doit :
1. Lire la bannière (stdout du pipe SSH)
2. Extraire le nonce
3. Calculer la preuve
4. Envoyer la preuve sur stdin

Cette **lecture bidirectionnelle** est impossible dans un simple pipe bash. Elle nécessite soit :
- Un script bash complexe avec des FIFOs nommés (fragile, non portable)
- Un client dédié (surcoût de développement et maintenance)
- Un outil comme `expect` (dépendance supplémentaire)

Or, le cas d'usage principal de SSH Frontière est justement l'automatisation simple en bash depuis des runners CI/CD.

### Analyse de la menace

Le nonce protège contre le **replay d'une preuve interceptée**. Mais dans le contexte SSH :

1. **Le canal SSH est déjà chiffré** (AES-256-GCM, ChaCha20-Poly1305) — un attaquant ne peut pas intercepter la preuve en transit
2. **L'authentification SSH** (clé Ed25519) a déjà lieu avant que SSH Frontière ne démarre — l'identité du client est établie
3. **Le modèle one-shot** (un processus par connexion) garantit qu'une preuve n'est utilisée qu'une seule fois par design

Le nonce ajoute une protection pertinente uniquement si SSH Frontière est exposé **sans SSH** (accès TCP direct, canal non chiffré) — un cas d'usage marginal mais documenté dans l'exercice d'alignement 001.

---

## Options

### Option A — Nonce désactivé par défaut, activable dans config.toml

Le nonce devient optionnel. Par défaut, l'authentification se fait par un SHA-256 simple du secret. Le nonce reste disponible en option pour les déploiements sans SSH.

### Option B — Nonce obligatoire, fournir un client dédié

Maintenir le nonce obligatoire et développer un binaire client (`ssh-frontiere-client`) qui gère la lecture bidirectionnelle. Le client serait distribué avec le serveur.

**Analyse** : cette option préserve la sécurité du protocole mais introduit un binaire supplémentaire (~500 LoC) à développer, tester, distribuer et maintenir. Chaque environnement (runner Forgejo, agent LLM, scripts maintenance) devrait installer ce client. Cela contredit le principe fondamental de SSH Frontière : l'automatisation se fait avec des outils standards (bash, SSH). Le client dédié transforme un composant serveur transparent en un système client-serveur couplé.

### Option C — Nonce dans la configuration SSH (authorized_keys)

Le nonce serait un paramètre statique défini dans `authorized_keys` plutôt que généré dynamiquement. Perd la protection anti-replay mais conserve la compatibilité pipe.

---

## Décision

### Option A — Nonce désactivé par défaut, activable dans config.toml

**Décision du BO (Julien, 2026-03-18)** : le nonce est désactivé par défaut.

### 1. Deux modes d'authentification

| Mode | Config | Proof | Protection |
|------|--------|-------|------------|
| **Simple** (défaut) | `challenge_nonce = false` (ou absent) | `proof = hex(SHA-256(secret))` | Authentification du secret, pas d'anti-replay (SSH assure la confidentialité) |
| **Nonce** | `challenge_nonce = true` | `proof = hex(SHA-256(XOR_encrypt(secret \|\| nonce, secret)))` | Authentification + anti-replay (pour accès sans SSH) |

### 2. Configuration TOML

```toml
[auth]
challenge_nonce = false   # défaut : pas de nonce

[auth.tokens.runner-forge]
secret = "b64:c2VjcmV0LXJ1bm5lci1mb3JnZQ=="
level = "ops"
tags = ["forgejo"]
```

Le champ `challenge_nonce` est un booléen sur la section `[auth]`, pas sur chaque token. Le mode d'authentification est une propriété du serveur (tous les clients utilisent le même mode). Cela simplifie la bannière et évite la complexité d'un mode mixte.

**Rétrocompatibilité config** : une configuration existante sans `challenge_nonce` fonctionne en mode simple (défaut = `false`). Aucune migration nécessaire. Les configurations qui avaient des tokens fonctionnent immédiatement — seul le calcul de la preuve côté client change.

### 3. Impact sur la bannière

**Mode simple** (pas de nonce) :
```
#> ssh-frontiere 0.x.x
+> capabilities rbac, session, help
#> type "help" for available commands
```

**Mode nonce** :
```
#> ssh-frontiere 0.x.x
+> capabilities rbac, session, help
+> challenge nonce=<hex_32>
#> type "help" for available commands
```

Le client sait quel mode utiliser grâce à la bannière :
- Pas de ligne `+> challenge nonce=` → mode simple (SHA-256 brut)
- Ligne `+> challenge nonce=<hex>` présente → mode nonce (challenge-response complet)

La capability reste `rbac` dans les deux cas. La présence/absence de la ligne `+> challenge nonce=` est le signal suffisant — pas besoin d'une capability dédiée `rbac-nonce` (information redondante).

**Sans token configuré** (pas de section `[auth]`) : pas de `rbac` dans les capabilities, pas de challenge — comportement inchangé.

### 4. Impact sur le code

#### `config.rs`

```rust
pub struct AuthConfig {
    #[serde(default)]
    pub challenge_nonce: bool,  // NOUVEAU — défaut false
    #[serde(default)]
    pub tokens: BTreeMap<String, TokenConfig>,
}
```

#### `crypto.rs`

Nouvelle fonction pour le mode simple :

```rust
/// Compute simple proof: SHA-256(secret) — no nonce
pub fn compute_simple_proof(secret: &[u8]) -> String {
    sha256(secret)
}

/// Verify simple proof (constant-time comparison)
pub fn verify_simple_proof(secret: &[u8], proof_hex: &str) -> bool {
    let expected = compute_simple_proof(secret);
    constant_time_eq(expected.as_bytes(), proof_hex.as_bytes())
}
```

Les fonctions existantes `compute_proof(secret, nonce)` et `verify_proof(secret, nonce, proof_hex)` restent inchangées.

#### `protocol.rs`

`write_banner()` : la ligne `+> challenge nonce=` n'est émise que si `challenge_nonce = true` **ET** des tokens sont configurés. La capability reste `rbac` dans les deux modes (la présence de la ligne `+> challenge` est le signal).

`validate_auth()` : le paramètre `nonce` passe de `&[u8]` à `Option<&[u8]>` :

```rust
pub fn validate_auth(
    &mut self,
    config: &Config,
    token_id: &str,
    proof_hex: &str,
    nonce: Option<&[u8]>,  // None = mode simple
) -> Result<TrustLevel, String> {
    // ...
    let valid = match nonce {
        Some(n) => crypto::verify_proof(&secret, n, proof_hex),
        None => crypto::verify_simple_proof(&secret, proof_hex),
    };
    // ...
}
```

#### `main.rs`

Le nonce n'est généré que si `challenge_nonce = true` :

```rust
let has_auth_tokens = config.auth.as_ref().is_some_and(|a| !a.tokens.is_empty());
let challenge_nonce = config.auth.as_ref().is_some_and(|a| a.challenge_nonce);
let nonce = if has_auth_tokens && challenge_nonce {
    Some(crypto::generate_nonce()?)
} else {
    None
};
```

En mode simple, `nonce` est `None`, et `validate_auth` reçoit `None` pour utiliser `verify_simple_proof`.

#### `bin/proof.rs`

Le binaire de preuve supporte les deux modes :

```
# Mode nonce (existant)
ssh-frontiere-proof --secret <secret> --nonce <hex_nonce>

# Mode simple (nouveau)
ssh-frontiere-proof --secret <secret>
```

Si `--nonce` est absent, le mode simple est utilisé : `proof = SHA-256(secret)`.

#### Boucle de session (`run_session_loop`)

La ré-authentification en session n'est nécessaire que pour **s'élever en droits** (changer de niveau ou de tags effectifs — autrement dit, changer de « compte »). Il est inutile de s'authentifier à chaque requête si le niveau d'accès et les tags sont déjà suffisants pour l'action demandée.

En mode simple, la preuve est toujours `SHA-256(secret)` (pas de nonce à régénérer). La preuve est **la même** à chaque `+auth` dans la session — ce n'est pas un problème car le canal SSH est déjà protégé contre le replay au niveau transport.

En mode nonce, le comportement actuel (régénération du nonce après `+auth` réussi, TODO-016) est préservé.

### 5. Cas d'usage par mode

| Cas d'usage | Mode recommandé | Justification |
|-------------|----------------|---------------|
| Runner Forgejo Actions (SSH) | Simple | Pipe bash, canal SSH chiffré |
| Agent LLM via SSH | Simple | Même raison |
| Maintenance automatisée (SSH) | Simple | Scripts bash standards |
| Accès TCP direct (pas de SSH) | Nonce | Canal non chiffré — double risque : MitM (l'applicatif ne peut rien y faire) + interception du secret (le nonce empêche le replay d'une preuve interceptée, sans révéler le secret) |
| Tests E2E Docker (SSH) | Simple (défaut) | Simplifier les scripts de test |
| Environnement haute sécurité | Nonce | Défense en profondeur supplémentaire |

### 6. Sécurité — analyse de risque

**Risque mode simple** : si un attaquant obtient `proof = SHA-256(secret)`, il ne peut pas retrouver `secret` (pré-image SHA-256). Mais la preuve est **déterministe** : la même preuve est réutilisable si le canal n'est pas protégé. Sur SSH, ce risque est nul (chiffrement du transport). Sans SSH, ce risque est réel → mode nonce requis.

**Clarification SHA-256 brut** : ce n'est pas du « stockage de mot de passe hashé » (où le sel et l'itération seraient critiques). Le secret est stocké **en clair** (base64) dans le config.toml — si un attaquant a accès au fichier, il a le secret brut, pas besoin de rainbow table. Le SHA-256 sert uniquement de **preuve de connaissance** sur le canal : le client prouve qu'il connaît le secret sans le transmettre en clair. Le sel est inutile dans ce contexte car l'attaquant potentiel n'a pas accès au hash — il n'a accès qu'au canal de transport (qui est chiffré par SSH).

**Risque mode nonce désactivé par défaut** : un administrateur pourrait déployer SSH Frontière en accès TCP direct sans activer le nonce. Atténuation : documentation claire dans le guide opérateur avec avertissement. SSH Frontière est conçu comme un **login shell** (invoqué par sshd) — l'utilisation sans SSH est un cas marginal documenté.

**Pas de downgrade attack** : le mode est fixé dans la configuration serveur (`config.toml`). Le client ne peut pas choisir ou demander un mode différent. Un attaquant MitM ne peut pas forcer un downgrade car le mode est décidé côté serveur avant tout échange.

### 7. Migration

**Aucune migration nécessaire pour les configs existantes.** Le `serde(default)` sur `challenge_nonce` garantit que les configs sans ce champ fonctionnent en mode simple.

**Impact bouchon bash** : le stub bash (`deploy/ssh-frontiere-stub.sh`) n'implémente pas l'authentification RBAC (il est pré-Phase 3). Aucune modification nécessaire du stub.

**Migration côté client** : les scripts qui calculaient une preuve avec nonce doivent être modifiés pour calculer `SHA-256(secret)` directement si le serveur passe en mode simple. L'impact est limité car le protocole avec nonce n'a été utilisé qu'en environnement de test (pas de consommateurs en production). Exemple bash :

```bash
# Ancien (avec nonce) — nécessitait lecture bidirectionnelle
NONCE=$(ssh user@host 2>/dev/null | grep "challenge" | ...)
PROOF=$(ssh-frontiere-proof --secret "$SECRET" --nonce "$NONCE")

# Nouveau (sans nonce) — simple
PROOF=$(echo -n "$SECRET" | sha256sum | cut -d' ' -f1)
echo -e "+ auth token=runner proof=$PROOF\ncommande arg=val\n." | ssh user@host
```

Ou avec le binaire proof :

```bash
PROOF=$(ssh-frontiere-proof --secret "$SECRET")
```

---

## Conséquences

### Positives

- **Débloque l'intégration Forgejo** : les runners peuvent s'authentifier via un simple pipe bash
- **Simplification drastique des scripts clients** : plus besoin de lecture bidirectionnelle
- **Rétrocompatibilité config** : les configurations existantes fonctionnent sans modification
- **Le nonce reste disponible** : les déploiements exposés sans SSH conservent la protection anti-replay
- **Pas de downgrade attack** : le mode est fixé côté serveur
- **Cohérence avec le modèle SSH** : SSH assure déjà la confidentialité et l'authenticité du canal — le nonce est une couche redondante dans ce contexte

### Négatives

- **Preuve déterministe en mode simple** : `SHA-256(secret)` est la même à chaque connexion. Si le secret fuit, toute connexion future est compromise (comme un mot de passe). Atténuation : le secret est déjà stocké en base64 dans le config.toml — la fuite du secret est le même risque qu'une fuite de mot de passe, indépendamment du nonce.
- **Complexité conditionnelle** : deux chemins de code (avec/sans nonce) dans `validate_auth`, `write_banner`, `main.rs`. Atténuation : les deux chemins sont simples (appel d'une fonction différente, pas de logique branching complexe).
- **Tests doublés** : chaque scénario d'auth doit être testé dans les deux modes. Atténuation : paramétrisation possible via fixtures config.

### Risques

- **Mauvais choix de mode par l'administrateur** : utiliser le mode simple sans SSH. Atténuation : documentation explicite, warning dans le guide opérateur.
- **Régression E2E** : les tests E2E existants (AUT-001 à AUT-011) utilisent le mode nonce. Ils doivent être mis à jour pour le mode par défaut (simple) et de nouveaux tests ajoutés pour le mode nonce. Impact : les 10+ scénarios AUT doivent être revus.

---

## Tests nécessaires

### Unitaires (crypto.rs)

1. `compute_simple_proof` — SHA-256 du secret = hex attendu
2. `verify_simple_proof` — preuve correcte → true
3. `verify_simple_proof` — preuve incorrecte → false
4. `compute_proof` (avec nonce) — inchangé, preuve existante toujours valide

### Unitaires (protocol.rs)

5. `validate_auth` mode simple (nonce=None) — preuve SHA-256 correcte → ok
6. `validate_auth` mode simple — preuve incorrecte → échec
7. `validate_auth` mode nonce (nonce=Some) — comportement inchangé
8. `validate_auth` mode simple — tags et level fusionnés correctement

### Unitaires (config.rs)

9. Config avec `challenge_nonce = true` → chargée correctement
10. Config avec `challenge_nonce = false` → chargée correctement
11. Config sans `challenge_nonce` → défaut false (rétrocompatibilité)

### Unitaires (bannière)

12. Bannière mode simple : pas de ligne `+> challenge nonce=`, `rbac` dans capabilities
13. Bannière mode nonce : ligne `+> challenge nonce=` présente, `rbac` dans capabilities
14. Bannière sans auth : pas de `rbac` dans capabilities (inchangé)

### Intégration

15. Mode simple : pipe complet (auth + commande) sans lecture nonce → exécution ok
16. Mode nonce : protocole complet avec nonce → exécution ok (régression)
17. Mode simple en session : re-auth sans nonce → ok

### Bin proof.rs

18. `ssh-frontiere-proof --secret X` (sans --nonce) → SHA-256(X)
19. `ssh-frontiere-proof --secret X --nonce Y` → preuve nonce (inchangé)

### E2E SSH

20. AUT-012 : authentification mode simple réussie (pipe bash)
21. AUT-013 : authentification mode simple avec mauvaise preuve → rejet
22. AUT-014 : authentification mode nonce activé → comportement existant préservé

---

## Attribution

- **Julien (BO)** : décision de rendre le nonce optionnel, cas d'usage Forgejo Actions, choix du mode simple par défaut (`proof = SHA-256(secret)`), analyse de la menace SSH vs TCP, exigence de rétrocompatibilité
- **Claude (PM/Tech Lead)** : analyse des options (client dédié vs nonce optionnel vs nonce statique), impact sur le code, détection via présence `+> challenge` dans la bannière, stratégie de migration, plan de tests
- **Agents Claude Code** : implémentation, tests
