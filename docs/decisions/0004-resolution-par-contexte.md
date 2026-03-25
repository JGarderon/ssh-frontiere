# ADR 0004 — Résolution par contexte : struct Contexte et modèle à 7 concepts

**Date** : 2026-03-15
**Statut** : Proposée
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : Exercice d'alignement 001, sections 2.0.1–2.0.7 (Modèle conceptuel)
**Voir aussi** : ADR 0001 (format configuration TOML — structure domaine/action, identités et secrets), ADR 0002 (modèle de secrets — champ `sensitive` dans ArgDef, protocole challenge-response), ADR 0003 (contrat d'interface — ordre domaine→action, format retour 4 champs, codes de sortie dans Decision::Reject)

---

## Contexte

L'exercice d'alignement 001 a défini 7 concepts fondamentaux qui structurent SSH Frontière : **Domaine**, **Action**, **Identité**, **Autorisation**, **Connexion**, **Règle**, **Contexte**.

Le concept central est le **Contexte** (2.0.7) : le croisement de tous les autres concepts au moment de l'invocation, point de résolution unique qui produit une décision (exécuter, refuser, différer).

Cette ADR formalise la traduction architecturale de ce modèle en structures Rust.

---

## Options

### Architecture de résolution

| Option | Description | Avantage | Inconvénient |
|--------|-------------|----------|-------------|
| A — God struct Contexte | Une struct unique qui rassemble toutes les données et résout | Simple, explicite, testable | Grosse struct, couplage |
| B — Pipeline de middlewares | Chaîne de fonctions qui transforment une requête | Découplé, extensible | Over-engineering pour un one-shot |
| C — Pattern matching direct | Match sur les tokens dans main(), sans struct intermédiaire | Minimal | Non-maintenable au-delà de 10 commandes |

### Organisation domaine → action

| Option | Description |
|--------|-------------|
| A — Flat (commandes à plat) | Pas de notion de domaine, chaque commande est un ID unique |
| B — Hiérarchique (domaine.action) | Les commandes sont organisées par domaine |
| C — Tags | Les commandes ont des tags, le domaine est un tag parmi d'autres |

---

## Décision

### God struct Contexte (option A) + Organisation hiérarchique (option B)

Conformément à l'exercice d'alignement 001 (section 2.0.7, décision Julien BO) :

> Le contexte se traduit naturellement en une struct unique en Rust. C'est volontairement un objet riche (« god struct ») au début du développement — on ne découpe pas prématurément. Les frontières naturelles émergeront de l'usage réel.

### Modèle de données

```rust
/// Concept 2.0.1 — Domaine : périmètre fonctionnel
struct Domain {
    id: String,
    description: String,
    actions: HashMap<String, Action>,
}

/// Concept 2.0.2 — Action : opération dans un domaine
struct Action {
    id: String,
    description: String,
    level: TrustLevel,
    timeout: u64,
    execute: String,         // transposition (commande système)
    args: Vec<ArgDef>,
}

/// Concept 2.0.3 — Identité : qui demande
struct Identity {
    level: TrustLevel,       // depuis --level (authorized_keys)
    ssh_client: Option<String>,  // IP:port depuis SSH_CLIENT
    fingerprint: Option<String>, // futur : fingerprint de la clé
}

/// Concept 2.0.4 — Autorisation : RBAC léger
enum TrustLevel { Read, Ops, Admin }

/// Concept 2.0.7 — Contexte : point de résolution
struct Context {
    // Entrée
    raw_command: String,
    identity: Identity,

    // Résolution
    domain: Option<Domain>,     // résolu depuis la commande
    action: Option<Action>,     // résolu depuis la commande
    args: HashMap<String, String>,

    // Environnement
    timestamp: String,          // ISO 8601
    pid: u32,
    config_path: String,

    // Décision (remplie par la résolution)
    decision: Decision,
}

enum Decision {
    Execute { command: String, args: Vec<String>, timeout: u64 },
    Reject { reason: String, code: i32 },
    // Defer { reason: String },  // futur Phase 3+
}
```

### Flux de résolution

```
1. Construire Identity (--level + SSH_CLIENT + env)
       │
2. Parser la commande brute (parseur grammatical + tokenize)
       │
3. Résoudre Domaine (1er token) + Action (2e token) (cf. ADR 0003 — domaine d'abord)
       │
4. Vérifier Autorisation (identity.level >= action.level)
       │
5. Valider les arguments (type enum, sensitive flag)
       │
6. Construire la Décision (Execute ou Reject)
       │
7. Si Execute : transposer et exécuter
       │
8. Logger le Contexte complet (JSON)
       │
9. Retourner le résultat (JSON stdout + code de sortie)
```

Chaque étape peut court-circuiter vers un `Reject`. Le Contexte accumule les informations à chaque étape et sert de structure de logging (toutes les données sont là).

### Règle = Domaine + Action en configuration

Le concept de **Règle** (2.0.6) se matérialise par la combinaison d'un domaine et d'une action dans la configuration TOML :

```toml
[domains.forgejo.actions.backup-config]   # ← c'est une Règle
level = "ops"                              # facette autorisation
timeout = 600                              # facette exécution
execute = "sudo /usr/local/bin/backup-config.sh {domain}"  # transposition
```

Il n'y a pas de struct `Rule` séparée — la règle est la conjonction de `Domain` + `Action` + les champs d'autorisation/exécution. C'est volontaire : le concept est riche mais la structure est plate.

### Connexion = mode ping/pong (MVP)

Le concept de **Connexion** (2.0.5) est implicite au MVP : chaque invocation est un ping/pong (1 connexion = 1 commande = 1 résultat). Il n'y a pas de struct `Connection` — la connexion est le processus lui-même.

Le mode session (connexion ouverte avec flux de commandes) est différé en Phase 3+. Quand il sera implémenté, une struct `Connection` ou `Session` encapsulera le flux stdin/stdout.

---

## Conséquences

### Positives

- Alignement direct entre le modèle conceptuel (7 concepts) et le code Rust
- Le Contexte comme point de résolution unique simplifie le raisonnement et le testing
- La god struct est explicite et assumée — pas de complexité cachée
- Le flux de résolution est linéaire et court-circuitable (chaque étape peut rejeter)
- Le logging a accès à toutes les données via le Contexte

### Négatives

- La god struct va grossir avec les phases futures (plage horaire, quota, chaînage)
- Le couplage est élevé au début — acceptable car le programme est petit (~800 LoC max par fichier)
- Les concepts Connexion et Règle n'ont pas de struct dédiée — cela peut créer de la confusion si on lit l'alignement sans lire le code

### Risques

- Si le Contexte dépasse la limite de taille confortable (~15 champs), il faudra le restructurer — mais l'alignement dit explicitement « les frontières naturelles émergeront de l'usage réel »
- Le matching domaine+action doit être déterministe : une même commande ne peut résoudre qu'une seule règle (collision interdite)

### Points de vigilance pour le refactoring futur

Le refactoring se fera quand :
1. Le Contexte dépasse 15 champs → extraire des sous-structs
2. Le mode session est implémenté → extraire Connection
3. Les règles deviennent conditionnelles (plage horaire, quota) → extraire Rule

---

## Attribution

- **Julien (BO)** : modèle à 7 concepts, god struct assumée, sens de conception fonctionnel→technique, mode connexion ping/pong vs session
- **Claude (PM/Tech Lead)** : traduction en structs Rust, flux de résolution, critères de refactoring, matérialisation Règle=Domaine+Action
- **Agents Claude Code** : implémentation, tests
