# ADR 0001 — Format de configuration TOML

**Date** : 2026-03-15
**Statut** : Proposée
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : Exercice d'alignement 001, sections 2.0.6 (Règle) et 3.2 (Extensibilité)
**Voir aussi** : ADR 0002 (secrets — champ `sensitive`, protocole challenge-response, masquage SHA-256 optionnel), ADR 0003 (contrat d'interface — ordre domaine→action, format retour 4 champs), ADR 0004 (struct Context et flux de résolution)

---

## Contexte

SSH Frontière doit lire sa configuration depuis un fichier externe pour rester extensible sans recompilation. La configuration décrit :
- Les **domaines** (périmètres fonctionnels : mastodon, forgejo, infra…)
- Les **actions** par domaine (backup-config, healthcheck, restart…)
- Les **règles** associées (niveau requis, timeout, transposition, arguments)
- Les **identités et niveaux de confiance** (comment le niveau est transmis, lien avec `authorized_keys`)
- Les **secrets** (arguments sensibles marqués `sensitive`, masquage SHA-256 optionnel, section `[auth]` réservée)
- Les paramètres globaux (log_file, default_timeout, default_level, mask_sensitive, limites de sortie)

L'exercice d'alignement 001 a établi que la configuration est **déclarative** (pas de scripts, pas de plugins). Chaque règle a deux facettes : autorisation et exécution.

La dépendance `toml` (crate Rust) doit être évaluée selon la matrice de la politique de dépendances du projet.

---

## Options

### Option A — TOML avec crate `toml`

Format lisible par les humains, commentaires natifs, syntaxe légère pour les tables imbriquées. Standard de facto en Rust (Cargo.toml).

**Structure proposée** :

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
log_level = "info"
default_timeout = 300
default_level = "ops"
mask_sensitive = false       # masquage SHA-256 des args sensibles (ADR 0002)
max_stdout_chars = 65536     # limite stdout (64 Ko)
max_stderr_chars = 16384     # limite stderr (16 Ko)
max_output_chars = 131072    # hard limit indépassable (128 Ko)

[domains.forgejo]
description = "Forge Git infrastructure"

[domains.forgejo.actions.backup-config]
description = "Sauvegarde la configuration Forgejo"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
args = []

[domains.forgejo.actions.backup-full]
description = "Sauvegarde complète (config + données)"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-full.sh {domain}"
args = []

[domains.infra.actions.healthcheck]
description = "Vérification de l'état des services"
level = "read"
timeout = 30
execute = "sudo /usr/local/bin/healthcheck.sh"
args = []

[domains.forgejo.actions.deploy]
description = "Déploiement avec tag de version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[[domains.forgejo.actions.deploy.args]]
name = "tag"
type = "enum"
values = ["latest", "stable", "canary"]
```

**Note sur le format `args`** : quand la liste est vide, on utilise `args = []`. Quand des arguments sont définis, chaque entrée est une table TOML avec `name`, `type`, et des champs spécifiques au type (`values` pour enum, `sensitive` pour les champs protégés — cf. ADR 0002).

**Évaluation crate `toml`** (v0.8.x, crates.io) :

| Critère | Note | Poids | Score |
|---------|------|-------|-------|
| Licence | MIT + Apache 2.0 | Éliminatoire | ✓ |
| Origine et gouvernance | Alex Crichton (anciennement Mozilla/Rust team), communauté US/mondiale | ×3 | 3.5/5 |
| Communauté | ~3300 stars GitHub, >200M téléchargements crates.io, utilisé par Cargo lui-même | ×2 | 5/5 |
| Fréquence de mise à jour | Releases régulières, activement maintenu | ×2 | 4.5/5 |
| Taille | ~50 Ko source, impact binaire modéré (~30-50 Ko avec serde) | ×3 | 4/5 |
| Dépendances transitives | `serde` (déjà présent), `toml_edit`, `toml_datetime`, `winnow` (parser) | ×3 | 3/5 |
| Fonctionnalités | Parsing + sérialisation TOML complète, intégration serde native | ×2 | 5/5 |
| Non-enfermement | Format TOML standardisé (spec v1.0), portabilité totale | ×1 | 5/5 |

**Score pondéré** : (3.5×3 + 5×2 + 4.5×2 + 4×3 + 3×3 + 5×2 + 5×1) / (3+2+2+3+3+2+1) = **4.0/5**

**Note sur l'origine** : Alex Crichton est US/Rust Foundation. Le crate est de facto le parser TOML officiel de l'écosystème Rust (utilisé par Cargo). Le risque supply-chain est comparable à celui de `serde` — accepter serde mais refuser toml serait incohérent.

**Dépendances transitives** : `winnow` (parser combinator, ~250 Ko source) est la principale sous-dépendance non déjà présente. C'est le compromis principal.

### Option B — JSON avec serde_json (déjà présent)

Aucune dépendance supplémentaire. Mais : pas de commentaires, syntaxe verbeuse pour les configurations imbriquées, erreur humaine facile (virgules, accolades).

### Option C — Format maison (clé=valeur)

Zéro dépendance. Mais : parser à écrire et maintenir, surface de bugs dans un composant de sécurité, pas de standard.

---

## Décision

**Option A — TOML avec crate `toml`.**

### Structure hiérarchique domaine → action

La configuration adopte une structure **domaine → action** qui reflète directement le modèle conceptuel à 7 concepts :

```
[global]                          → paramètres globaux
[domains.<id>]                    → concept Domaine (2.0.1)
[domains.<id>.actions.<id>]       → concept Action (2.0.2)
  level = "ops"                   → concept Autorisation (2.0.4, facette)
  timeout = 600                   → concept Règle (2.0.6, facette exécution)
  execute = "..."                 → transposition (2.0.2)
  args = [...]                    → validation arguments
```

### Validation au chargement

Le chargement est fail-fast : toute erreur de configuration empêche le démarrage. Validations :

1. **Syntaxe TOML** : parsing via serde
2. **Complétude** : chaque action a un `execute`, un `level` valide, un `timeout` > 0
3. **Cohérence** : les placeholders `{arg}` dans `execute` correspondent aux entrées de `args`
4. **Domaines** : au moins un domaine défini, chaque domaine a au moins une action
5. **Arguments enum** : chaque enum a au moins une valeur autorisée

### Extensibilité

- Ajouter un domaine = ajouter une section `[domains.<id>]`
- Ajouter une action = ajouter `[domains.<id>.actions.<id>]`
- Pas de recompilation, pas de redéploiement du binaire
- Le format permet d'ajouter des champs futurs (plage horaire, quota, chaînage) sans casser la compatibilité

### Identités, niveaux et secrets dans la configuration

La configuration TOML décrit les **règles** (domaines + actions) mais ne gère pas directement les identités ni les secrets d'authentification. Ces aspects sont traités par l'ADR 0002 (modèle de secrets à trois niveaux) :

- **Identité** : établie par SSH (`authorized_keys` + `command="--level=..."`) — hors de la configuration TOML. Le fichier de config ne contient pas de liste d'utilisateurs ni de clés SSH.
- **Niveaux de confiance** : chaque action déclare le `level` requis (`read`, `ops`, `admin`). Le niveau de l'identité est passé par `--level` au démarrage, pas stocké dans la config.
- **Secrets d'authentification** (Phase 3+) : quand le RBAC par token sera implémenté (protocole challenge-response, cf. ADR 0002), les empreintes de tokens seront stockées dans une section `[auth]` du TOML, encodées en base64. Le format est réservé :

```toml
# Futur (Phase 3+) — section auth avec empreintes de tokens
[auth.tokens]
runner-forge = { hash = "b64:YTJmNDg2...", level = "ops" }
agent-claude = { hash = "b64:NWE3YjJk...", level = "read" }
```

- **Arguments sensibles** : le champ `sensitive = true` dans la définition d'un argument (cf. ADR 0002, Niveau 1) déclenche optionnellement le masquage SHA-256 dans les logs. Ce masquage est **configurable, pas imposé** :

```toml
[global]
mask_sensitive = true   # active le masquage SHA-256 des args sensibles (défaut: false)
```

Pour le détail du modèle de secrets et du protocole d'authentification challenge-response, voir **ADR 0002**.

### Chemin de configuration

- Défaut : `/etc/ssh-frontiere/config.toml`
- Override : `--config <path>` ou variable `SSH_FRONTIERE_CONFIG`
- Permissions recommandées : `root:root 640` (ou groupe dédié `ssh-frontiere`)

---

## Conséquences

### Positives

- Format lisible et commentable, adapté à la maintenance humaine
- Structure domaine/action alignée sur le modèle conceptuel
- Extensibilité sans recompilation
- Intégration serde transparente avec les structs Rust
- Validation stricte au chargement (fail-fast)

### Négatives

- Ajout d'une dépendance (`toml` + `winnow` + `toml_edit` + `toml_datetime`)
- Impact binaire estimé : +50-80 Ko en release musl (à vérifier)
- La crate `toml` est d'origine US (Rust Foundation), ce qui est noté mais accepté vu son statut dans l'écosystème

### Risques

- Si la crate `toml` est compromise, c'est l'ensemble de l'écosystème Rust qui est touché (risque systémique, pas spécifique à ssh-frontiere)
- Le format TOML ne supporte pas nativement les inclusions de fichiers — si le besoin multi-fichier émerge (Phase 3+), il faudra l'implémenter manuellement

---

## Attribution

- **Julien (BO)** : choix TOML vs JSON, validation du format, politique de dépendances
- **Claude (PM/Tech Lead)** : évaluation matricielle du crate, structure domaine/action, validation au chargement
- **Agents Claude Code** : implémentation, tests
