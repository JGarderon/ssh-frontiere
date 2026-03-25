+++
title = "Architecture"
description = "Conception technique de SSH-Frontière : langage, modules, protocole, dépendances"
date = 2026-03-24
weight = 3
+++

# Architecture et conception

## Pourquoi Rust

SSH-Frontière est écrit en Rust pour trois raisons :

1. **Sécurité mémoire** : pas de buffer overflow, pas de use-after-free, pas de null pointer. Pour un composant de sécurité qui tourne en tant que login shell, c'est critique.

2. **Binaire statique** : compile avec la cible `x86_64-unknown-linux-musl` (d'autres cibles possibles sans garantie de fonctionnement), le binaire fait ~1 Mo et n'a aucune dépendance système. On le copie sur le serveur et c'est prêt.

3. **Performance** : le programme démarre, valide, exécute et meurt en millisecondes. Pas de runtime, pas de garbage collector, pas de JIT.

## Synchrone et éphémère

SSH-Frontière est un programme **synchrone et one-shot**. Pas de daemon, pas d'async, pas de Tokio.

Le cycle de vie est simple :
1. `sshd` authentifie la connexion SSH par clé
2. `sshd` fork et exécute `ssh-frontiere` comme login shell
3. `ssh-frontiere` valide et exécute la commande
4. Le processus se termine

Chaque connexion SSH crée un nouveau processus. Pas d'état partagé entre connexions, pas de problème de concurrence.

## Structure du code

Le code est organisé en modules avec des responsabilités claires :

| Module | Responsabilité |
|--------|----------------|
| `main.rs` | Point d'entrée, aplatissement des arguments, appel à l'orchestrateur |
| `orchestrator.rs` | Flux principal : bannière, en-têtes, commande, réponse, boucle session |
| `config.rs` | Structures de configuration TOML, validation fail-fast |
| `protocol.rs` | Protocole d'en-têtes : parseur, bannière, auth, session, body |
| `crypto.rs` | SHA-256 (implémentation FIPS 180-4), base64, nonce, challenge-response |
| `dispatch.rs` | Parsing de commande (guillemets, `key=value`), résolution, RBAC |
| `chain_parser.rs` | Parseur de chaînes de commandes (opérateurs `;`, `&`, `\|`) |
| `chain_exec.rs` | Exécution des chaînes : séquence stricte (`;`), permissive (`&`), rattrapage (`\|`) |
| `discovery.rs` | Commandes `help` et `list` : découverte des domaines et actions |
| `logging.rs` | Logging JSON structuré, masquage des arguments sensibles |
| `output.rs` | Réponse JSON, codes de sortie |
| `lib.rs` | Exposition de `crypto` pour le binaire proof et helpers de fuzz |

Chaque module a son fichier de tests (`*_tests.rs`) dans le même répertoire.

Un binaire auxiliaire `proof` (`src/bin/proof.rs`) permet de calculer les proofs d'authentification pour les tests E2E et l'intégration avec des clients.

## Protocole d'en-têtes

SSH-Frontière utilise un protocole texte sur stdin/stdout. Les préfixes diffèrent selon la direction :

**Client vers serveur (stdin) :**

| Préfixe | Rôle |
|---------|------|
| `+ ` | **Configuré** : directives (`auth`, `session`, `body`) |
| `# ` | **Commente** : ignorés par le serveur |
| *(texte brut)* | **Commande** : `domaine action [arguments]` |
| `.` *(seul sur une ligne)* | **Fin de bloc** : termine un bloc de commande |

**Serveur vers client (stdout) :**

| Préfixe | Rôle |
|---------|------|
| `#> ` | **Commente** : bannière, messages informatifs |
| `+> ` | **Configuré** : capabilities, challenge nonce |
| `>>> ` | **Répond** : réponse JSON finale |
| `>> ` | **Stdout** : sortie standard en streaming (ADR 0011) |
| `>>! ` | **Stderr** : sortie d'erreur en streaming |

### Flux de connexion

```
CLIENT                                  SERVEUR
  |                                        |
  |  <-- bannière + capabilities --------  |   #> ssh-frontiere 0.1.0
  |                                        |   +> capabilities rbac, session, help, body
  |                                        |   +> challenge nonce=a1b2c3...
  |                                        |   #> type "help" for available commands
  |                                        |
  |  --- +auth (optionnel) ------------>   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session (optionnel) --------->   |   + session keepalive
  |                                        |
  |  --- commande (texte brut) -------->   |   forgejo backup-config
  |  --- fin de bloc ------------------->  |   .
  |  <-- streaming stdout -------------    |   >> Backup completed
  |  <-- réponse JSON finale -----------   |   >>> {"status_code":0,"status_message":"executed",...}
  |                                        |
  |  (si session keepalive)                |
  |  --- commande 2 ------------------->   |   infra healthcheck
  |  --- fin de bloc ------------------->  |   .
  |  <-- réponse JSON 2 ---------------   |   >>> {"status_code":0,...}
  |  --- fin de session (bloc vide) --->   |   .
  |  <-- session closed ----------------   |   #> session closed
```

### Réponse JSON

Chaque commande produit une réponse JSON finale sur une seule ligne, préfixée par `>>>`. La sortie standard et d'erreur sont envoyées en streaming via `>>` et `>>!` :

```
>> Backup completed
>>> {"command":"forgejo backup-config","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

- `stdout`/`stderr` = `null` dans la réponse JSON finale : la sortie a été envoyée en streaming via `>>` et `>>!`
- Pour les commandes non exécutées (rejet, erreur config), `stdout` et `stderr` sont aussi `null`

### Protocole body

L'en-tête `+body` permet de transmettre du contenu multiligne vers le processus enfant via stdin. Quatre modes de délimitation :

- `+body` : lit jusqu'à une ligne contenant uniquement `.` (point)
- `+body size=N` : lit exactement N octets
- `+body stop="DELIMITEUR"` : lit jusqu'à une ligne contenant le délimiteur
- `+body size=N stop="DELIMITEUR"` : premier délimiteur atteint (taille ou marqueur) termine la lecture

## Configuration TOML

Le format de configuration est du TOML déclaratif. Choix documenté dans l'ADR 0001 :

- **Pourquoi TOML** : lisible par les humains, typage natif, standard dans l'écosystème Rust, pas d'indentation significative (contrairement à YAML), plus expressif que JSON pour la configuration.
- **Pourquoi pas YAML** : indentation significative source d'erreurs, types implicites dangereux (`on`/`off` → booléen), spécification complexe.
- **Pourquoi pas JSON** : pas de commentaires, verbeux, pas conçu pour la configuration humaine.

La configuration est **validée au chargement** (fail-fast) : syntaxe TOML, complétude des champs, cohérence des placeholders, au moins un domaine, au moins une action par domaine, valeurs enum non vides.

## Politique de dépendances

SSH-Frontière a une politique de **zéro dépendance non vitale**. Chaque crate externe doit être justifiée par un besoin réel.

### Dépendances actuelles

3 dépendances directes, ~20 dépendances transitives :

| Crate | Usage |
|-------|-------|
| `serde` + `serde_json` | Sérialisation JSON (logging, réponses) |
| `toml` | Chargement de la configuration TOML |

### Matrice d'évaluation

Avant d'ajouter une dépendance, elle est évaluée sur 8 critères pondérés (note /5) : licence (éliminatoire), gouvernance (x3), communauté (x2), fréquence de mise à jour (x2), taille (x3), dépendances transitives (x3), fonctionnalités (x2), non-enfermement (x1). Score minimum : 3.5/5.

### Audit

- `cargo deny` vérifie les licences et les vulnérabilités connues
- `cargo audit` cherche les failles dans la base RustSec
- Sources autorisées : crates.io uniquement

## Comment le projet a été conçu

SSH-Frontière a été développé en phases successives (1 à 9, avec des phases intermédiaires 2.5 et 5.5), piloté par des agents Claude Code avec une méthodologie TDD systématique :

| Phase | Contenu |
|-------|---------|
| 1 | Dispatcher fonctionnel, config TOML, RBAC 3 niveaux |
| 2 | Configuration production, scripts d'opérations |
| 2.5 | SHA-256 FIPS 180-4, BTreeMap, timeout gracieux |
| 3 | Protocole d'en-têtes unifié, auth challenge-response, sessions |
| 4 | Tests E2E SSH Docker, nettoyage code, intégration forge |
| 5 | Tags de visibilité, filtrage horizontal par tokens |
| 5.5 | Nonce optionnel, arguments nommés, binaire proof (inclut la phase 6, fusionnée) |
| 7 | Guide configuration, dry-run `--check-config`, help sans préfixe |
| 8 | Types d'erreur structurés, clippy pedantic, cargo-fuzz, proptest |
| 9 | Protocole body, arguments libres, max_body_size, code sortie 133 |

Le projet a été conçu par :
- **Julien Garderon** (BO) : concept, spécifications fonctionnelles, choix Rust, nom du projet
- **Claude superviseur** (PM/Tech Lead) : analyse technique, architecture
- **Agents Claude Code** : implémentation, tests, documentation

Où l'humain et la machine travaillent ensemble, mieux, plus vite, avec davantage de sécurité.
