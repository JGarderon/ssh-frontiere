# ADR 0008 — Tags de visibilité et filtrage horizontal

**Date** : 2026-03-17
**Statut** : Accepted (validée par Julien BO, 2026-03-17)
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : Exercice d'alignement 001 (§3.4 — visibilité = autorisation), ADR 0004 (contexte et résolution), ADR 0006 (protocole d'entêtes et auth RBAC)
**Voir aussi** : ADR 0001 (format configuration TOML), ADR 0009 (arguments nommés et valeurs par défaut)

---

## Contexte

Le RBAC actuel de SSH Frontière est **vertical** : les niveaux de confiance (`read < ops < admin`) contrôlent l'accès aux actions par hiérarchie. Un utilisateur `ops` peut exécuter toutes les actions `read` et `ops`, dans **tous** les domaines.

Ce modèle est insuffisant pour les déploiements multi-consommateurs :

- Le runner Forgejo (niveau `ops`) peut exécuter les backups Mastodon — ce n'est pas souhaitable
- Un agent LLM avec token `ops` voit et peut exécuter toutes les actions `ops` de tous les domaines
- Il n'y a aucun moyen de restreindre un consommateur à un périmètre fonctionnel sans créer un niveau dédié

L'exercice d'alignement 001, section 3.4, acte déjà le principe : « le RBAC contrôle non seulement l'exécution mais aussi la **visibilité** — on n'expose que ce qui peut être fait ».

Le besoin est un **filtrage horizontal** (par périmètre fonctionnel) en complément du filtrage vertical (par niveau). Les tags sont le mécanisme identifié.

### Pourquoi pas des niveaux configurables ?

La piste « niveaux de confiance configurables » (permettre à l'administrateur de définir ses propres niveaux au-delà de `read/ops/admin`) a été évaluée et **rejetée** :

1. **Les tags couvrent le besoin** : le filtrage horizontal (tags) + le filtrage vertical (3 niveaux) donnent une matrice d'autorisations suffisamment fine. `read+forgejo` ≠ `read+mastodon` ≠ `ops+forgejo`.

2. **3 niveaux suffisent** : `read` (consultation), `ops` (opérations), `admin` (tout). Ces 3 niveaux couvrent la taxonomie naturelle des opérations d'infrastructure. Ajouter des niveaux intermédiaires (ex: `deploy`, `monitor`) revient à modéliser des rôles — or les tags font déjà ce travail.

3. **Complexité disproportionnée** : des niveaux configurables impliquent un `TrustLevel` dynamique (plus d'enum Rust), une hiérarchie ordonnée à valider au chargement, des comparaisons `>=` sur des types non compilés, et une migration de tous les tests. Le ratio coût/bénéfice est défavorable.

4. **Risque de sécurité** : un enum compilé est prouvable par le compilateur. Un niveau dynamique introduit une classe de bugs (niveaux mal ordonnés, niveaux manquants, comparaisons incorrectes) dans un composant de sécurité.

5. **Cohérence avec l'alignement 001** : « Sécurité > Flexibilité » (§5.3). Les niveaux compilés sont plus sûrs que des niveaux configurables.

---

## Options

### Option A — Tags sur les actions et les tokens RBAC

Chaque action peut déclarer des tags optionnels. Chaque token RBAC (`tags = [...]` dans config.toml) peut déclarer des tags. L'autorisation vérifie le niveau **ET** l'intersection de tags.

### Option B — Domaines isolés par identité

Chaque identité déclare les domaines accessibles. Plus simple mais moins granulaire (pas de filtrage par action au sein d'un domaine).

### Option C — ACL explicites par identité

Liste exhaustive `identité → [domaine.action, ...]`. Maximal mais verbeux et non-maintenable à l'échelle.

---

## Décision

### Option A — Tags sur les actions et les tokens RBAC

Les tags sont des **étiquettes** libres (chaînes alphanumériques + tiret) qui permettent de restreindre la visibilité et l'exécution par intersection.

### 1. Règles d'intersection

| Tags action | Tags effectifs (via token RBAC) | Résultat |
|-------------|-------------------------------|----------|
| `[]` (aucun) | `[]` (aucun) | **Autorisé** — action publique, identité non restreinte |
| `[]` (aucun) | `["forgejo"]` | **Autorisé** — action publique, accessible à tous |
| `["forgejo"]` | `[]` (aucun) | **Refusé** — action restreinte, identité sans tag correspondant |
| `["forgejo"]` | `["forgejo"]` | **Autorisé** — intersection non vide |
| `["forgejo"]` | `["mastodon"]` | **Refusé** — intersection vide |
| `["forgejo", "infra"]` | `["forgejo"]` | **Autorisé** — intersection non vide (au moins un tag commun) |

**Résumé** :
- Action sans tag → accessible à toute identité du bon niveau (comportement actuel préservé)
- Action avec tags → accessible uniquement aux identités ayant **au moins un tag en commun**
- Identité sans tag → accès uniquement aux actions sans tag

Ce modèle est **rétrocompatible** : une configuration sans aucun tag fonctionne exactement comme avant.

### 2. Impact sur la visibilité (découverte)

Le principe de l'alignement 001 §3.4 s'étend aux tags : **un domaine sans aucune action visible disparaît du `help` et du `list`**.

La découverte (`help`, `list`) filtre par :
1. Niveau de confiance (existant) : `identity.level >= action.level`
2. Tags (nouveau) : intersection non vide OU action sans tag

Un consommateur ne voit **que** les actions qu'il peut exécuter. Un domaine dont toutes les actions sont masquées par les tags n'apparaît pas.

### 3. Configuration TOML

#### Tags sur les actions

```toml
[domains.forgejo.actions.backup-config]
description = "Sauvegarde la configuration Forgejo"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
tags = ["forgejo"]

[domains.mastodon.actions.backup-db]
description = "Sauvegarde la base de données Mastodon"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-db.sh {domain}"
tags = ["mastodon"]

[domains.infra.actions.healthcheck]
description = "Vérification globale des services"
level = "read"
timeout = 30
execute = "sudo /usr/local/bin/healthcheck.sh"
# Pas de tags → accessible à tous les utilisateurs read+
```

#### authorized_keys — uniquement `--level` et `--config`

Le `authorized_keys` ne porte **que** le niveau de confiance et le chemin de configuration — comme aujourd'hui :

```
command="ssh-frontiere --level=ops --config=/etc/ssh-frontiere/config.toml",restrict ssh-ed25519 AAAA... runner@forge
```

Les tags ne sont **pas** dans `authorized_keys`. Le config.toml est la source de vérité unique pour les tags (sur les actions et sur les tokens RBAC). Cette séparation garantit qu'on peut modifier les tags d'un consommateur en éditant uniquement le config.toml, sans toucher aux `authorized_keys`.

#### Tags sur les tokens RBAC

```toml
[auth.tokens.runner-forge]
secret = "b64:c2VjcmV0LXJ1bm5lci1mb3JnZQ=="
level = "ops"
tags = ["forgejo", "infra"]

[auth.tokens.agent-mastodon]
secret = "b64:c2VjcmV0LWFnZW50LW1hc3RvZG9u"
level = "ops"
tags = ["mastodon"]
```

#### Source des tags d'identité : tokens RBAC uniquement

Les tags d'une identité proviennent **exclusivement** des tokens RBAC définis dans le config.toml :

- **Avant toute authentification `+auth`** : tags effectifs = `[]` → l'identité n'a accès qu'aux actions sans tags (publiques)
- **Après `+auth` avec un token tagué** : tags effectifs = tags du token
- **En session, authentifications multiples** : tags effectifs = union des tags de tous les tokens authentifiés

```
# Avant +auth
tags_effectifs = []

# Après +auth avec runner-forge (tags: ["forgejo", "infra"])
tags_effectifs = ["forgejo", "infra"]

# Après second +auth avec agent-mastodon (tags: ["mastodon"])
tags_effectifs = ["forgejo", "infra", "mastodon"]
```

Le config.toml est la **source de vérité unique** pour les tags. On peut modifier les tags d'un consommateur en éditant le config.toml, sans toucher aux `authorized_keys`. Le token peut **ajouter** des tags au périmètre, mais pas en retirer (union cumulative — même logique que l'élévation de niveau via token).

### 4. Impact sur le code

#### Structs modifiées

```rust
// config.rs
pub struct ActionConfig {
    // ... champs existants ...
    #[serde(default)]
    pub tags: Vec<String>,   // NOUVEAU
}

pub struct TokenConfig {
    pub secret: String,
    pub level: TrustLevel,
    #[serde(default)]
    pub tags: Vec<String>,   // NOUVEAU
}

// dispatch.rs — Identity INCHANGÉ
// Pas de champ tags sur Identity — les tags viennent du contexte d'auth (tokens RBAC)
pub struct Identity {
    pub level: TrustLevel,
    pub ssh_client: Option<String>,
}
```

#### Fonctions modifiées

```rust
// dispatch.rs — Identity::from_args()
// INCHANGÉ — ne parse que --level et --config (pas de --tags)

// dispatch.rs — check_authorization()
// Ajouter paramètre effective_tags: &[String] (depuis AuthContext)
// Vérification d'intersection de tags après le check de niveau

// dispatch.rs — help_full(), help_target(), list_actions()
// Ajouter paramètre effective_tags: &[String] (depuis AuthContext)
// Filtrage par tags en plus du filtrage par niveau existant

// protocol.rs — AuthContext
// Nouveau champ effective_tags: Vec<String>, initialisé à vec![]
// Lors de +auth valide : effective_tags = union(effective_tags, token.tags)
// main.rs récupère &auth_context.effective_tags et le passe aux fonctions dispatch
```

#### Nouvelle fonction

```rust
/// Vérifie que l'identité a au moins un tag en commun avec l'action
/// Retourne true si l'action n'a pas de tags (publique)
fn check_tags(identity_tags: &[String], action_tags: &[String]) -> bool {
    if action_tags.is_empty() {
        return true; // Action publique
    }
    identity_tags.iter().any(|t| action_tags.contains(t))
}
```

### 5. Validation au chargement

- Les tags sont des chaînes alphanumériques + tiret (même contrainte que les noms de tokens)
- Un tag vide `""` est une erreur de configuration
- Pas de limite de nombre de tags (pragmatisme > règle arbitraire)
- Les doublons dans une liste de tags sont dédupliqués silencieusement au chargement
- Les tags sont normalisés en minuscules au chargement (`Forgejo` → `forgejo`)
- **Warning de cohérence** : au chargement, si un tag défini sur un token RBAC n'apparaît dans aucune action, un warning est émis sur stderr (pas une erreur — l'admin peut prévoir des tags pour des actions futures). Ce warning aide à détecter les typos

### 6. Impact sur le protocole

Aucun changement de protocole. Les tags sont un mécanisme de configuration serveur — le client ne voit pas et ne manipule pas les tags directement. Le client voit uniquement le résultat : les actions visibles dans `help`/`list`, et les rejets pour les actions non autorisées.

**Conséquence mode one-shot** : un consommateur qui veut accéder à des actions taguées **doit** s'authentifier via `+auth` avant d'envoyer sa commande `$`, même en mode one-shot (sans `+session keepalive`). Sans `+auth`, les tags effectifs sont vides et seules les actions publiques (sans tags) sont accessibles. C'est cohérent avec le protocole Phase 3 : `+auth` est la seule voie pour acquérir des tags.

Le message de rejet pour un défaut de tag est le même code (128 = rejeté) mais avec un message distinct :

```json
{"status_code": 128, "status_message": "rejected: access denied (tag mismatch)", "stdout": null, "stderr": null}
```

**Décision de sécurité** : le message ne liste pas les tags requis ni les tags de l'identité. Un attaquant ne doit pas pouvoir énumérer les tags par essai-erreur. Le message est opaque : « tag mismatch ».

### 7. Impact sur le logging

Le log JSON inclut les tags dans le contexte :

```json
{
    "event": "rejected",
    "identity_tags": ["forgejo"],
    "action_tags": ["mastodon"],
    "reason": "tag mismatch",
    ...
}
```

Les tags sont tracés intégralement dans les logs (pas de masquage — les tags ne sont pas des secrets).

---

## Conséquences

### Positives

- **Filtrage horizontal** : chaque consommateur est restreint à son périmètre fonctionnel
- **Rétrocompatibilité totale** : une configuration sans tags fonctionne identiquement à l'actuel
- **Visibilité cohérente** : `help`/`list` ne montrent que ce qui est accessible (principe alignement 001 §3.4)
- **Simplicité** : intersection de tags = une fonction de 3 lignes, pas de nouvelle dépendance
- **Extensible** : les tags sont des chaînes libres — l'administrateur définit sa taxonomie
- **Rejet argumenté des niveaux configurables** : simplifie la feuille de route v3

### Négatives

- Ajout d'un champ `tags` dans 2 structs config (`ActionConfig`, `TokenConfig`) + `effective_tags` dans `AuthContext`
- Le message de rejet opaque (« tag mismatch ») peut compliquer le debug — atténué par le log détaillé côté serveur
- La fusion des tags (union) en cas de `+auth` ajoute une logique d'état dans le contexte d'authentification

### Risques

- **Tag typo** : un tag mal orthographié (`forgeo` au lieu de `forgejo`) bloque silencieusement l'accès. Atténuation : validation au chargement (tags non vides, format valide), mais pas de vérification croisée tags-actions ↔ tags-identités (ce serait trop contraignant et l'admin peut avoir des raisons de prévoir des tags futurs)
- **Explosion combinatoire** : si l'admin définit 50 tags sur 50 actions, la configuration devient illisible. Atténuation : bonne documentation (guide opérateur) avec recommandation de 3-5 tags maximum
- **Sécurité de l'union** : la fusion des tags via `+auth` élargit le périmètre. C'est voulu (même logique que l'élévation de niveau via token) mais doit être documenté comme un comportement explicite. **Le guide opérateur doit avertir** qu'un token mal protégé donne accès à plus de domaines que l'identité de base.

---

## Tests nécessaires

### Unitaires (dispatch.rs)

1. `check_tags` — action sans tag → toujours autorisé
2. `check_tags` — identité sans tag, action avec tags → refusé
3. `check_tags` — intersection non vide → autorisé
4. `check_tags` — intersection vide → refusé
5. `check_tags` — tags multiples, un seul en commun → autorisé
6. `check_authorization` avec tags — niveau OK + tags OK → autorisé
7. `check_authorization` avec tags — niveau OK + tags KO → refusé
8. `check_authorization` avec tags — niveau KO + tags OK → refusé (le niveau prime)

### Unitaires (protocol.rs — AuthContext)

9. `AuthContext` — avant `+auth`, effective_tags = `[]`
10. `AuthContext` — après `+auth` avec token tagué, effective_tags = tags du token
11. `AuthContext` — `+auth` multiples en session, effective_tags = union des tags
12. `AuthContext` — token sans tags, effective_tags inchangés

### Unitaires (config.rs)

13. Chargement config avec `tags` sur actions → OK
14. Chargement config sans `tags` → OK (rétrocompatibilité)
15. Chargement config avec tag vide → erreur
16. Chargement config avec tags sur tokens RBAC → OK
17. Tags dédupliqués au chargement
18. Tags normalisés en minuscules au chargement
19. Warning si tag de token absent de toutes les actions (vérifier stderr)

### Unitaires (découverte)

20. `help` filtre par tags — action avec tags non matching → invisible
21. `help` filtre par tags — domaine entièrement masqué → invisible
22. `list` filtre par tags — cohérent avec help
23. `help <domaine>` — domaine masqué → erreur « unknown domain »

### Intégration

24. Scénario complet : sans `+auth` → seules les actions sans tags accessibles
25. Scénario complet : `+auth` avec token tagué forgejo → actions forgejo + publiques visibles et exécutables
26. Scénario session avec `+auth` multiples : fusion des tags → périmètre élargi

### E2E SSH

27. SEC-015 : tags empêchent l'exécution cross-domaine (sans auth → actions publiques uniquement)
28. SEC-016 : tags sur token RBAC appliqués correctement
29. AUT-011 : auth avec token tagué → actions taguées accessibles

---

## Attribution

- **Julien (BO)** : concept des tags de visibilité, cas d'usage runner Forgejo vs Mastodon, principe « visibilité = autorisation » (alignement 001 §3.4), rejet des niveaux configurables (les tags suffisent), correction critique : tags uniquement dans config.toml (tokens RBAC), pas dans authorized_keys (source de vérité unique)
- **Claude (PM/Tech Lead)** : modèle d'intersection (matrice tags action × identité), fusion par union pour `+auth`, message de rejet opaque, impact sur le code, plan de tests
- **Agents Claude Code** : implémentation, tests
