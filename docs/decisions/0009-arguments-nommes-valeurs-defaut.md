# ADR 0009 — Arguments nommés et valeurs par défaut

**Date** : 2026-03-17
**Statut** : Accepted (validée par Julien BO, 2026-03-17)
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : ADR 0001 (format configuration TOML — structure args), ADR 0003 (contrat d'interface — format `domaine action [args]`), ADR 0004 (résolution par contexte)
**Voir aussi** : ADR 0008 (tags de visibilité)

---

## Contexte

Le système d'arguments actuel de SSH Frontière est **strictement positionnel** :

```
$ forgejo deploy latest
```

L'utilisateur doit connaître l'ordre exact des arguments. Ce modèle a deux limitations :

1. **Lisibilité** : avec 2+ arguments, l'ordre n'est pas évident. `forgejo deploy latest true` — qu'est-ce que `true` ? Le dry-run ? Le verbose ? L'utilisateur (humain ou LLM) doit consulter `help` pour chaque commande.

2. **Rigidité** : tous les arguments sont obligatoires. Si une action a 3 arguments et que les 2 derniers ont des valeurs « évidentes » dans 90% des cas, l'utilisateur doit quand même les spécifier à chaque fois.

L'exercice d'alignement 001 §3.2 pose l'extensibilité comme priorité 2 : « ajouter une commande = ajouter une section dans la configuration ». Le principe fondamental de SSH Frontière est que **toute la configuration vit dans le config.toml** : le binaire lit le config.toml et s'adapte. Les arguments nommés et les valeurs par défaut s'inscrivent dans cette logique — l'administrateur déclare les arguments, leurs contraintes et leurs defaults dans le config.toml, et le binaire résout automatiquement. C'est le même principe que pour les domaines, les actions et les tags : la configuration est déclarative et centralisée.

---

## Options

### Arguments nommés

| Option | Syntaxe | Avantage | Inconvénient |
|--------|---------|----------|-------------|
| A — `key=value` | `forgejo deploy tag=latest` | Naturel, pas d'ambiguïté `=` | Le `=` dans une valeur (`foo=bar=baz`) nécessite une règle de parsing |
| B — `--key value` | `forgejo deploy --tag latest` | Convention CLI | Collision avec les `--` de ssh-frontiere (`--level`, `--config`) |
| C — `key:value` | `forgejo deploy tag:latest` | Pas de collision `=` | Le `:` est moins conventionnel, confusion avec protocole |

### Valeurs par défaut

| Option | Mécanisme | Avantage | Inconvénient |
|--------|-----------|----------|-------------|
| X — Champ `default` dans ArgDef | `default = "latest"` | Simple, déclaratif | Chaque argument avec default est optionnel |
| Y — Arguments optionnels explicites | `required = false` + `default` | Plus explicite | Plus verbeux |
| Z — Surcharge globale | Defaults dans `[global]` | Centralisé | Confusion scope |

---

## Décision

### Arguments nommés : option A (`key=value`), exclusivement nommés

On change de paradigme : les arguments sont **exclusivement nommés**. L'ancien format positionnel est abandonné. On est en cycle court, pas en production stable avec des consommateurs externes — la rupture est assumée.

#### Syntaxe

```
$ forgejo deploy tag=latest
$ mastodon backup mode=full target=s3
$ forgejo deploy tag=canary verbose=true
```

#### Règles de parsing

1. Chaque token d'argument est un **argument nommé** : split sur le **premier** `=` — tout avant est le nom, tout après est la valeur. `param=foo=bar` → name=`param`, value=`foo=bar`
2. Un token sans `=` est une **erreur de syntaxe** — les arguments positionnels ne sont pas supportés
3. Le nom (partie avant le `=`) doit correspondre à un nom d'argument défini dans le config.toml. Un nom inconnu est une **erreur** (argument inconnu)
4. Un argument fourni en double est une **erreur**
5. Un token `key=` (valeur vide après le `=`) est valide — la valeur est la chaîne vide

#### Pas de collision avec le parseur grammatical

Le parseur grammatical (ADR 0003, principe Julien BO) reste le gardien :
- La grammaire est `domaine action [args]` — le `=` dans un argument est du **contenu**, pas de la syntaxe shell
- Le `=` n'est interprété qu'**après** la tokenisation (`tokenize_with_quotes`), lors de la résolution des arguments (`resolve_arguments`)
- Aucune liste noire, aucun caractère interdit — le parseur grammatical fait son travail

### Valeurs par défaut : option X (champ `default` dans ArgDef)

#### Configuration TOML

Les arguments sont déclarés comme une table TOML où **le nom de l'argument est la clé** :

```toml
[domains.forgejo.actions.deploy]
description = "Deploiement avec tag de version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }
verbose = { type = "enum", values = ["true", "false"], default = "false" }
```

L'avantage de cette syntaxe :
- Le nom de l'argument **est** la clé TOML — pas de champ `name` redondant dans un tableau anonyme
- Le nom est garanti unique par construction (clé de table)
- C'est plus lisible et compact que `[[domains.xxx.actions.yyy.args]]`

#### Règles

1. Un argument avec `default` est **optionnel** : s'il n'est pas fourni, la valeur par défaut est utilisée
2. Un argument sans `default` est **obligatoire** : s'il n'est pas fourni, c'est une erreur
3. La valeur par défaut est soumise aux mêmes validations que les valeurs fournies (enum check pour les enums). Pour les arguments de type `string`, toute valeur par défaut est acceptée

```toml
[domains.example.actions.test.args]
target = { type = "string" }                                        # obligatoire (pas de default)
mode = { type = "enum", values = ["fast", "full"], default = "fast" } # optionnel (avec default)
```

#### Interaction arguments nommés + valeurs par défaut

L'utilisateur ne fournit que les arguments qui diffèrent du défaut :

```
$ forgejo deploy                           # tag=latest (défaut), verbose=false (défaut)
$ forgejo deploy tag=canary                # tag=canary, verbose=false (défaut)
$ forgejo deploy tag=canary verbose=true   # tout explicite
```

### Impact sur le code

#### Structs modifiées

```rust
// config.rs
pub struct ArgDef {
    #[serde(rename = "type")]
    pub arg_type: String,
    #[serde(default)]
    pub values: Option<Vec<String>>,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub default: Option<String>,   // NOUVEAU
}
```

Le champ `name` disparaît de `ArgDef` — le nom est la clé de la `BTreeMap`. Le champ `args` d'`ActionConfig` passe de `Vec<ArgDef>` à `BTreeMap<String, ArgDef>` :

```rust
// config.rs
pub struct ActionConfig {
    // ... champs existants ...
    #[serde(default)]
    pub args: BTreeMap<String, ArgDef>,  // clé = nom de l'argument
}
```

`BTreeMap` garantit l'ordre déterministe (cohérent avec TODO-005).

#### Fonctions modifiées

```rust
// dispatch.rs — resolve_command()
// 1. Pour chaque token, split sur le premier '=' → (name, value)
// 2. Token sans '=' → erreur de syntaxe
// 3. Vérifier que name correspond à un ArgDef → sinon erreur (argument inconnu)
// 4. Vérifier pas de doublon
// 5. Appliquer les defaults pour les arguments manquants
// 6. Vérifier qu'il ne reste pas d'argument obligatoire non fourni
// 7. Valider les valeurs (enum check, comme avant)
```

**Extraction recommandée** : la logique de résolution des arguments sera extraite dans une fonction `resolve_arguments()` appelée par `resolve_command()`, pour respecter le seuil de 60 lignes par fonction (CLAUDE.md). `resolve_command()` reste l'orchestrateur.

Le reste du pipeline (check_authorization, transpose_command, execute_command) ne change pas — ils reçoivent toujours un `HashMap<String, String>` résolu.

#### Validation au chargement

Ajouts à `Config::validate_action()` :

1. Vérifier que les valeurs `default` passent la validation enum (si applicable)
2. Vérifier que les noms d'arguments sont conformes (alphanumériques + tiret/underscore)

La vérification d'unicité des noms est garantie par construction (`BTreeMap` — clé unique).

### Impact sur le protocole

Aucun changement de protocole. La syntaxe `key=value` est du contenu dans la ligne `$ domaine action [args]`. Le protocole d'entêtes (ADR 0006) n'est pas affecté.

### Impact sur la découverte

Le `help` inclut les informations de default. Le format est cohérent avec la `BTreeMap` : le nom de l'argument est la **clé** de l'objet JSON, pas un champ interne :

```json
{
    "args": {
        "tag": {
            "type": "enum",
            "values": ["latest", "stable", "canary"],
            "default": "latest"
        },
        "verbose": {
            "type": "enum",
            "values": ["true", "false"],
            "default": "false"
        }
    }
}
```

Le champ `default` apparaît quand il est défini. L'absence de `default` indique un argument obligatoire. Cela permet aux agents LLM de savoir quels arguments omettre.

### Impact sur le logging

Les arguments résolus (y compris les defaults appliqués) sont loggés normalement. Un champ `defaults_applied` est inclus **en mode `log_level = "debug"`** uniquement, pour ne pas alourdir les logs en production :

```json
{
    "event": "executed",
    "domain": "forgejo",
    "action": "deploy",
    "args": {"tag": "latest", "verbose": "false"},
    "defaults_applied": ["tag", "verbose"],
    ...
}
```

En mode `info` (défaut), seul le champ `args` résolu est loggé — sans distinction entre valeurs fournies et defaults.

---

## Conséquences

### Positives

- **Ergonomie** : les commandes courantes deviennent plus courtes (`forgejo deploy` au lieu de `forgejo deploy tag=latest verbose=false`)
- **Lisibilité** : `forgejo deploy tag=canary verbose=true` est auto-documenté
- **LLM-friendly** : les arguments nommés éliminent l'ambiguïté positionnelle, et les defaults réduisent la verbosité
- **Cohérence config.toml** : les arguments, leurs contraintes et leurs defaults vivent dans la configuration — même logique que domaines, actions et tags
- **Cohérence avec le parseur grammatical** : le `=` est du contenu, pas de la syntaxe shell — pas de nouvelle surface d'attaque
- **Simplicité du parsing** : pas de mode positionnel, pas de règle de mixage, pas d'ambiguïté nom-connu/positionnel
- **Unicité par construction** : le nom est la clé TOML (`BTreeMap`), pas de validation de doublons à coder

### Négatives

- **Rupture** avec le format positionnel actuel — les scripts existants doivent migrer vers `key=value`. Acceptable en cycle court, pas de consommateurs externes stables.
- Complexité accrue de `resolve_command()` : passage de 5 lignes (validation count + enum check) à ~25 lignes (parsing nommé + defaults + validation)
- Le split sur le premier `=` introduit une subtilité : `param=value=with=equals` est valide (nom=`param`, valeur=`value=with=equals`)

### Risques

- **Injection via le nom** : un nom d'argument comme `__proto__` ou un nom très long pourrait être problématique. Atténuation : le nom est validé contre les clés de la `BTreeMap<String, ArgDef>` — tout nom inconnu est rejeté. Pas de traitement dynamique des noms.
- **Verbosité** : pour les actions à un seul argument, `forgejo deploy tag=latest` est plus verbeux que `forgejo deploy latest`. Atténuation : avec un default, `forgejo deploy` suffit dans la majorité des cas.

---

## Tests nécessaires

### Unitaires (dispatch.rs — parsing nommé)

1. Argument nommé simple : `tag=latest` → name=tag, value=latest
2. Argument nommé avec `=` dans la valeur : `param=a=b` → name=param, value=a=b
3. Argument nommé avec valeur entre guillemets : `tag="latest stable"` → name=tag, value=latest stable
4. Argument nommé avec valeur vide : `tag=` → name=tag, value=""
5. Token sans `=` → erreur de syntaxe (positionnel refusé)
6. Nom d'argument inconnu → erreur (argument inconnu)
7. Argument en double (`tag=a tag=b`) → erreur
8. Plusieurs arguments nommés valides : `tag=canary verbose=true`

### Unitaires (dispatch.rs — defaults)

9. Action avec defaults, aucun arg fourni → defaults appliqués
10. Action avec defaults, arg explicite → override le default
11. Action avec arg obligatoire manquant → erreur
12. Default enum avec valeur invalide → erreur au chargement
13. Mix obligatoire + optionnel : obligatoire fourni, optionnel omis → default appliqué
14. Mix obligatoire + optionnel : obligatoire omis → erreur

### Unitaires (config.rs — validation)

15. Chargement config avec inline tables `args` → OK
16. Default enum hors `values` → erreur au chargement
17. Noms d'arguments conformes (alphanumériques + tiret/underscore) → OK

### Unitaires (découverte)

18. `help` — args sérialisés avec nom comme clé JSON (cohérent BTreeMap)
19. `help` — champ `default` présent quand défini, absent sinon

### Intégration

20. Commande complète avec args nommés → exécution OK
21. Commande avec defaults → transposition correcte (defaults substitués)
22. Commande avec argument positionnel (sans `=`) → erreur

### E2E SSH

23. PRO-011 : commande avec arguments nommés
24. PRO-012 : commande avec valeurs par défaut omises
25. SEC-017 : argument positionnel rejeté (pas de `=`)

---

## Attribution

- **Julien (BO)** : besoin d'arguments nommés pour l'ergonomie, cas d'usage `forgejo deploy tag=latest`, valeurs par défaut déclaratives, suppression du positionnel (rupture assumée en cycle court), syntaxe inline tables TOML (nom = clé), recentrage sur config.toml comme source de vérité unique
- **Claude (PM/Tech Lead)** : syntaxe `key=value` (split premier `=`), `BTreeMap<String, ArgDef>` pour unicité par construction, validation au chargement, analyse des risques
- **Agents Claude Code** : implémentation, tests
