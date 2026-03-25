# Audit de simplification — SSH-Frontière

Date : 2026-03-25

## Résumé exécutif

Le code source de SSH-Frontière (~3 470 LOC sur 12 fichiers) est globalement bien structuré avec des types explicites, un découpage modulaire clair et une gestion d'erreurs rigoureuse (zéro `unwrap` en runtime). Le ratio est d'environ **70% bien pensé / 30% simplifiable**. Les points principaux à adresser sont : `dispatch.rs` qui mélange parsing et exécution de processus, `protocol.rs` qui accumule trop de responsabilités (669 LOC, le plus gros module), et 4 fonctions portant `#[allow(clippy::too_many_lines)]` qui signalent une complexité reconnue mais non traitée.

## Analyse par module

### src/main.rs (52 LOC)

**Structures** : Aucune.

**Fonctions > 35 LOC** : Aucune. `flatten_args()` (L30-L52, 22 LOC) est concise et claire.

**Cohésion** : Point d'entrée minimal — `flatten_args` + appel orchestrateur + exit.

**Verdict** : ✅ Bien pensé

---

### src/lib.rs (77 LOC)

**Structures** : Aucune (module `fuzz_helpers` conditionnel).

**Fonctions > 35 LOC** : Aucune. Toutes les fonctions de `fuzz_helpers` sont des wrappers fins.

**Cohésion** : Clair — expose `crypto` pour le binaire `proof` et des wrappers pour le fuzzing.

**Verdict** : ✅ Bien pensé

---

### src/config.rs (426 LOC)

**Structures** :
- `ConfigError` (enum, 3 variants) : Clair et bien typé (Io, Parse, Validation).
- `TrustLevel` (enum, 3 variants) : Bien choisi avec `PartialOrd`/`Ord` pour les comparaisons RBAC. Implémentations `Display` et `FromStr` propres.
- `ArgDef` (struct, 5 champs) : Tous nécessaires. Le champ `arg_type` renommé depuis `type` (mot réservé Rust) via `#[serde(rename)]` est un bon compromis.
- `ActionConfig` (struct, 8 champs) : Tous utilisés. `is_visible_to()` est bien placée ici comme méthode — c'est une règle métier de la config.
- `DomainConfig` (struct, 2 champs) : Minimal, bien.
- `GlobalConfig` (struct, 13 champs dont 3 fantômes) : **Problème.** Les champs `_log_level`, `_default_level`, `_mask_sensitive` existent uniquement pour la compatibilité TOML (désérialisés mais jamais lus). C'est du code mort documenté. **Action** : supprimer ces champs. Si la rétrocompatibilité TOML est nécessaire, utiliser `#[serde(flatten)]` avec `HashMap<String, toml::Value>` pour capturer les champs inconnus sans polluer la struct.
- `TokenConfig` (struct, 3 champs) : Bien.
- `AuthConfig` (struct, 2 champs) : Bien.
- `Config` (struct, 3 champs) : Bien.

**Note** : 10 fonctions `default_*` (L150-L188, ~40 LOC) pour les valeurs par défaut serde. C'est du boilerplate incompressible — serde exige des fonctions, pas des constantes.

**Fonctions > 35 LOC** :
- `fn validate` (L233-L319, 86 LOC) : Fait 4 choses distinctes — (1) vérifier domaines non vides, (2) valider actions+tags, (3) valider tokens auth avec détection tags orphelins, (4) vérifier limites de sortie. **Action** : extraire `validate_auth_tokens(&self)` (~40 LOC) et `validate_output_limits(&self)` (~10 LOC) comme méthodes privées. `validate()` deviendrait un orchestrateur de ~30 LOC.
- `fn validate_action_fields` (L347-L425, 78 LOC) : Fait 3 choses — (1) valider max_body_size, (2) valider les arguments (noms + enum), (3) vérifier la cohérence des placeholders `{arg}` dans `execute`. **Action** : extraire `validate_placeholders(execute, args)` (~20 LOC).

**Cohésion** : Bonne — tout concerne la configuration. Les fonctions de validation sont dans le bon module.

**Verdict** : ⚠️ Simplifiable — `GlobalConfig` avec 3 champs fantômes, `validate()` et `validate_action_fields()` à découper

---

### src/orchestrator.rs (432 LOC)

**Structures** :
- `SessionContext` (struct, 6 champs) : Bon regroupement des paramètres de session (évite 6 paramètres individuels dans `run_session_loop`).

**Fonctions > 35 LOC** :
- `fn run` (L30-L224, 194 LOC) : **Le plus gros point de complexité du projet.** Porte `#[allow(clippy::too_many_lines)]`. 9 étapes séquentielles numérotées (config → identity → nonce → banner → headers → comments → auth → help-shortcircuit → command → session). Chaque étape est bien séparée par des commentaires, mais la fonction est trop longue pour être lue d'un coup. **Action** : extraire les étapes 3-4 (nonce + banner) dans `setup_connection()` et les étapes 7-8 (auth initiale + help shortcircuit) dans `handle_initial_auth()`. Cela ramènerait `run()` sous 80 LOC.
- `fn run_session_loop` (L291-L388, 97 LOC) : Match sur 6 variantes de `SessionInput`. La branche `CommandBlock` et la branche `Auth` font chacune ~25 LOC. Acceptable mais à la limite. **Point important** : la logique auth failure/lockout/ban est **dupliquée** entre `run()` (L122-L142) et `run_session_loop()` (L332-L370). **Action** : extraire une fonction commune `handle_auth_result()`.
- `fn generate_session_id` (L227-L254, 27 LOC) : Le formatage UUID v4 est tortueux — slicing hex + `from_str_radix` pour un seul nibble variant. Fonctionnel mais plus complexe que nécessaire. **Action** : manipuler les bytes directement (`bytes[6] = (bytes[6] & 0x0F) | 0x40` pour version, `bytes[8] = (bytes[8] & 0x3F) | 0x80` pour variant) avant la conversion hex.

**Cohésion** : Bonne — orchestration du protocole. Seule la duplication auth est un point faible.

**Verdict** : ⚠️ Simplifiable — `run()` à 194 LOC, logique auth dupliquée

---

### src/dispatch.rs (619 LOC)

**Structures** :
- `DispatchError` (enum, 9 variants) : Bien typé, couvre tous les cas d'erreur du dispatch. L'implémentation `Display` est propre — 3 variants partagent le pattern `write!(f, "{msg}")` (L46-L48).
- `Identity` (struct, 2 champs) : Minimal, approprié.
- `ExecuteResult` (enum, 5 variants) : Clair et exhaustif.
- `StreamLine` (enum, 2 variants) : Privée, simple.

**Fonctions > 35 LOC** :
- `fn tokenize_with_quotes` (L116-L154, 38 LOC) : Juste au-dessus du seuil. Machine à états classique avec guillemets simples et doubles. Flow clair, pas de simplification évidente.
- `fn resolve_arguments` (L196-L271, 75 LOC) : Trois phases distinctes — parsing key=value, application des defaults, validation enum. **Action** : extraire `validate_enum_values(args, arg_defs)` (~20 LOC).
- `fn execute_command` (L353-L528, 175 LOC) : **Deuxième plus grosse fonction du projet.** Porte `#[allow(clippy::too_many_lines)]`. Gère le spawn, le thread stdin (body), les threads stdout/stderr, la boucle d'événements avec timeout, et le cleanup (kill). C'est de la plomberie système légitime, mais ce n'est **pas du dispatch** — c'est de l'exécution de processus.

**Cohésion** : **Point faible majeur.** Ce module mélange deux responsabilités très différentes :
1. **Parsing et résolution de commandes** (`parse_command`, `tokenize_with_quotes`, `resolve_command`, `resolve_arguments`, `check_authorization`, `check_tags`, `transpose_command`) — ~270 LOC de logique métier
2. **Exécution de processus** (`execute_command`, `write_stream_line`, `drain_channel`, `kill_process`, `send_signal_to_group`, `ExecuteResult`, `StreamLine`) — ~250 LOC de plomberie système avec threads, channels, signaux Unix

**Action** : extraire un module `src/executor.rs` contenant la partie exécution. Le module `dispatch.rs` garderait la logique métier (parsing, résolution, autorisation, transposition) et descendrait à ~370 LOC.

**Verdict** : **À restructurer** — deux responsabilités distinctes dans un même module

---

### src/discovery.rs (143 LOC)

**Structures** : Aucune.

**Fonctions > 35 LOC** :
- `fn help_target` (L55-L94, 39 LOC) — à la limite. Deux recherches séquentielles (d'abord comme domaine, puis comme action). Flow linéaire et clair.

**Cohésion** : Excellente — uniquement les commandes de découverte JSON (help, list).

**Verdict** : ✅ Bien pensé

---

### src/chain_parser.rs (337 LOC)

**Structures** :
- `ChainError` (enum, 2 variants) : Minimal, suffisant.
- `CommandNode` (enum, 3 variants) : AST clair. **Point faible** : le booléen dans `Sequence(Vec<CommandNode>, bool)` pour strict/permissif est opaque. Il faut se souvenir que `true = strict (;)` et `false = permissif (&)`. **Action** : remplacer par un `enum SequenceMode { Strict, Permissive }`.
- `Token` (enum, 5 variants) : Interne, approprié.

**Fonctions > 35 LOC** :
- `fn tokenize_block` (L100-L184, 84 LOC) : Tokenizer classique avec gestion des guillemets et parenthèses. La complexité est inhérente au problème. Le `while i < len` avec indexation dans un `Vec<char>` est idiomatique pour un parseur avec lookahead potentiel. Pas de simplification évidente.
- `fn parse_sequence` (L239-L282, 43 LOC) : Parseur récursif descendant. Les branches `Semicolon` (L249-L259) et `Ampersand` (L260-L269) sont quasiment identiques — seul `ops.push(true)` vs `ops.push(false)` diffère. **Action** : fusionner en un seul bras `Token::Semicolon | Token::Ampersand` avec `let strict = matches!(token, Token::Semicolon)`.

**Cohésion** : Excellente — uniquement le parseur AST.

**Verdict** : ✅ Bien pensé (point mineur : `bool` → enum pour `Sequence`)

---

### src/chain_exec.rs (376 LOC)

**Structures** : Aucune.

**Fonctions > 35 LOC** :
- `fn execute_chain` (L24-L69, 45 LOC) : Récursion sur l'AST. Flow clair, pattern match propre. Acceptable.
- `fn execute_single_command` (L74-L201, 127 LOC) : Porte `#[allow(clippy::too_many_lines)]`. Pipeline séquentiel (parse → built-in? → resolve → authorize → transpose → execute → respond). Chaque étape est un bloc match/if avec early return. **Action** : extraire la phase 5 "transpose + execute + respond" dans `execute_and_respond()` (~50 LOC) et la gestion built-in (L107-L120) dans la fonction `handle_builtin_chain` existante.
- `fn write_help_text` (L243-L363, 120 LOC) : Porte `#[allow(clippy::too_many_lines)]`. Deux branches principales (vue d'ensemble ~50 LOC vs détail domaine ~45 LOC) séparées par un `if tokens.len() == 1`. **Action** : extraire `write_help_overview()` et `write_help_domain()`. La fonction principale ne ferait plus que router vers l'un ou l'autre + émettre la réponse finale.

**Cohésion** : Bonne — moteur d'exécution des chaînes. `log_command` et `write_help_text` sont logiquement liés à l'exécution.

**Verdict** : ⚠️ Simplifiable — deux fonctions > 100 LOC à découper

---

### src/protocol.rs (670 LOC)

**Structures** :
- `ProtocolLine` (enum, 5 variants) : Clair.
- `BodyMode` (enum, 4 variants) : Bien typé, couvre ADR 0012.
- `Directive` (enum, 6 variants) : Clair. `Unknown(String)` assure l'extensibilité.
- `HeadersResult` (struct, 5 champs) : Tous utilisés.
- `AuthContext` (struct, 5 champs) : **Mal placé.** L'implémentation de `validate_auth` (L432-L482) est de la logique métier d'authentification qui dépend de `crypto`. Elle n'a rien de protocolaire — c'est du RBAC.
- `ProtocolError` (enum, 5 variants) : Clair et exhaustif.
- `SessionInput` (enum, 5 variants) : Clair.

**Fonctions > 35 LOC** :
- `fn read_headers` (L368-L413, 45 LOC) : Machine à états propre. Pas de simplification évidente.
- `fn validate_auth` (L432-L482, 50 LOC) : Logique correcte. Le `if token.level > self.base_level { token.level } else { self.base_level }` (L459-L463) pourrait être `self.base_level.max(token.level)` puisque `TrustLevel` implémente `Ord`.
- `fn read_command_block` (L496-L531, 35 LOC) : Pile au seuil. Flow linéaire, acceptable.
- `fn read_session_input` (L594-L634, 40 LOC) : Machine à états avec `pending_body_mode`, propre.
- `fn write_banner` (L321-L362, 41 LOC) : Séquence d'écritures, acceptable.

**Note** : les 5 fonctions d'écriture (`write_response`, `write_stdout_line`, `write_stderr_line`, `write_comment`, `write_banner`) suivent toutes le même pattern `writeln!` + `flush` + `map_err`. C'est répétitif (~50 LOC de boilerplate) mais chaque fonction a un format de préfixe distinct (`>>>`, `>>`, `>>!`, `#>`), donc une macro serait du sur-engineering.

**Cohésion** : **Point faible.** Ce module de 670 LOC (le plus gros) regroupe :
1. Types du protocole (ProtocolLine, BodyMode, Directive, HeadersResult, ProtocolError, SessionInput) — ~95 LOC
2. Parseur de lignes (parse_line, parse_directive, parse_body_params, parse_kv) — ~100 LOC
3. Lecture de body (read_body et 3 variantes) — ~115 LOC
4. Bannière et écriture (write_banner, write_response, write_stdout_line, write_stderr_line, write_comment) — ~75 LOC
5. Lecture d'entêtes et sessions (read_headers, read_command_block, read_session_input) — ~115 LOC
6. **Auth context et validation** (AuthContext, validate_auth, is_locked_out) — ~70 LOC
7. **Ban command + IP extraction** (execute_ban_command, extract_ip_from_ssh_client) — ~35 LOC

Les groupes 1-5 sont cohérents (protocole). Les groupes 6 et 7 sont des responsabilités distinctes :
- `AuthContext` fait de la crypto et du RBAC → devrait être dans un module `auth.rs`
- `execute_ban_command` exécute un processus système → rien à voir avec le protocole

**Action** : extraire `AuthContext` + `validate_auth` + `is_locked_out` dans `src/auth.rs`. Déplacer `execute_ban_command` + `extract_ip_from_ssh_client` vers `orchestrator.rs` (seul appelant). Protocol.rs descendrait à ~565 LOC.

**Verdict** : **À restructurer** — `AuthContext` et `execute_ban_command` à extraire

---

### src/crypto.rs (295 LOC)

**Structures** : Aucune (constantes `K`, `H0` pour SHA-256).

**Fonctions > 35 LOC** :
- `fn sha256_bytes` (L30-L99, 69 LOC) : Implémentation SHA-256 FIPS 180-4. La complexité est **inhérente** à l'algorithme — aucune simplification possible sans sacrifier la lisibilité. Les commentaires `// PANIC-SAFE` sont bien placés.
- `fn base64_decode` (L127-L164, 37 LOC) : Décodeur base64 standard, borderline à 37 LOC. Acceptable.

**Cohésion** : Excellente — primitives cryptographiques pures, zéro dépendance externe, zéro effet de bord (sauf `generate_nonce` qui lit `/dev/urandom`).

**Verdict** : ✅ Bien pensé

---

### src/logging.rs (156 LOC)

**Structures** :
- `LogEntry` (struct, 12 champs) : Beaucoup de champs `Option`. C'est le pattern "bag of optional fields" — fonctionnel mais pas élégant. Cependant, la sérialisation JSON avec `#[serde(skip_serializing_if)]` rend le résultat propre. Les builders `.with_domain()`, `.with_action()`, etc. sont un bon pattern. **Acceptable en l'état.**

**Fonctions > 35 LOC** : Aucune. La plus longue est `epoch_days_to_ymd` (L98-L131, 33 LOC) — algorithme naïf (boucle année par année) mais correct et appelé une seule fois par entry de log. Choix assumé pour éviter la dépendance `chrono`.

**Cohésion** : Bonne — logging JSON + formatage timestamp.

**Verdict** : ✅ Bien pensé

---

### src/output.rs (66 LOC)

**Structures** :
- `Response` (struct, 5 champs) : Clair, avec 3 constructeurs nommés (`rejected`, `streamed`, `timeout`). Le constructeur direct est aussi utilisé dans `chain_exec.rs` pour les cas spéciaux (exit, help). Propre.

**Fonctions > 35 LOC** : Aucune.

**Cohésion** : Excellente — réponse JSON + codes de sortie + helpers stderr.

**Verdict** : ✅ Bien pensé

---

## Inventaire des `#[allow(clippy::too_many_lines)]`

4 fonctions portent cet attribut dans le code. Chaque `#[allow]` est une reconnaissance explicite de dette technique :

| Fonction | Module | LOC | Seuil hard (60) dépassé de |
|----------|--------|-----|---------------------------|
| `run()` | orchestrator.rs | 194 | +134 (×3.2) |
| `execute_command()` | dispatch.rs | 175 | +115 (×2.9) |
| `execute_single_command()` | chain_exec.rs | 127 | +67 (×2.1) |
| `write_help_text()` | chain_exec.rs | 120 | +60 (×2.0) |

**Total : 616 LOC de fonctions reconnues comme trop longues**, soit ~18% du code source.

---

## Corrections prioritaires

| # | Module | Type | Description | Impact |
|---|--------|------|-------------|--------|
| 1 | dispatch.rs | Restructuration | Extraire l'exécution de processus (`execute_command`, `kill_process`, `StreamLine`, `drain_channel`, `write_stream_line`, `send_signal_to_group`, `ExecuteResult`) dans un module `executor.rs`. dispatch.rs garde le parsing, la résolution et l'autorisation. | Elevé — séparation des responsabilités, testabilité |
| 2 | protocol.rs | Restructuration | Extraire `AuthContext` + `validate_auth` + `is_locked_out` dans un module `auth.rs`. Déplacer `execute_ban_command` + `extract_ip_from_ssh_client` vers `orchestrator.rs`. | Elevé — protocol.rs passe de 670 à ~565 LOC |
| 3 | orchestrator.rs | Simplification | Découper `run()` (194 LOC) en sous-fonctions : `setup_connection()` pour nonce+bannière, `handle_initial_auth()` pour la phase auth+help shortcircuit. Extraire la logique auth failure/lockout/ban dupliquée entre `run()` et `run_session_loop()` dans une fonction commune. | Moyen — lisibilité, DRY |
| 4 | chain_exec.rs | Simplification | Découper `write_help_text()` (120 LOC) en `write_help_overview()` et `write_help_domain()`. Extraire la phase execute+respond de `execute_single_command()` (127 LOC). | Moyen — fonctions sous 60 LOC |
| 5 | config.rs | Simplification | Découper `validate()` (86 LOC) en sous-méthodes (`validate_auth_tokens`, `validate_output_limits`). Extraire `validate_placeholders()` de `validate_action_fields()` (78 LOC). | Moyen — lisibilité |
| 6 | config.rs | Nettoyage | Supprimer les 3 champs fantômes de `GlobalConfig` (`_log_level`, `_default_level`, `_mask_sensitive`). Si la rétrocompatibilité TOML est nécessaire, utiliser `#[serde(flatten)]` avec un `HashMap<String, toml::Value>`. | Faible — code mort |
| 7 | chain_parser.rs | Simplification | Remplacer le `bool` dans `CommandNode::Sequence(Vec<CommandNode>, bool)` par un `enum SequenceMode { Strict, Permissive }`. Fusionner les branches dupliquées Semicolon/Ampersand dans `parse_sequence()`. | Faible — lisibilité |
| 8 | protocol.rs | Simplification | Dans `validate_auth()`, remplacer le if/else par `self.base_level.max(token.level)` (TrustLevel implémente Ord). | Faible — micro-simplification |
| 9 | orchestrator.rs | Simplification | Simplifier `generate_session_id()` : manipuler les bytes directement pour les bits version/variant du UUID v4, puis convertir en hex, au lieu du slicing hex + `from_str_radix`. | Faible — clarté |

## Recommandations de long terme

1. **Respecter le seuil de 60 LOC par fonction.** Les 4 `#[allow(clippy::too_many_lines)]` représentent 616 LOC de dette technique reconnue (18% du code). Le fait qu'elles soient annotées est positif — pas de suppression silencieuse de warnings — mais elles doivent être traitées avant d'en accumuler davantage.

2. **Un module = une responsabilité.** Le pattern actuel où `dispatch.rs` fait à la fois du parsing et de l'exécution de processus Unix, ou `protocol.rs` fait à la fois du parsing protocole et de l'authentification crypto, crée du couplage invisible. Le split en modules plus fins (executor, auth) faciliterait les tests unitaires ciblés et la navigation.

3. **Privilégier les types expressifs sur les booléens.** Le `bool` dans `Sequence` et les différents flags booléens (`strict`, `diagnostic`, `session_mode`) gagneraient à être des enums nommés quand le contexte n'est pas immédiatement évident à la lecture.

4. **Centraliser la logique d'authentification.** Actuellement dispersée entre `orchestrator.rs` (appel + gestion lockout), `protocol.rs` (AuthContext + validate_auth) et `crypto.rs` (verify_proof). Un module `auth.rs` regroupant le contexte et la validation simplifierait le raisonnement sur la sécurité.

## Conclusion à l'attention de Julien

Le code est **solide sur les fondamentaux** : zéro unwrap en runtime, types d'erreur explicites, parseur grammatical propre, crypto pure sans dépendance. L'architecture en couches (config → protocole → dispatch → exécution) est saine.

Les **deux restructurations prioritaires** (extraire `executor.rs` de dispatch et `auth.rs` de protocol) apporteraient le plus de valeur pour un effort modéré. Elles ne changent aucun comportement — c'est du réarrangement de code existant.

Les **simplifications de fonctions** (découper les 4 fonctions > 100 LOC) sont moins urgentes mais empêchent l'accumulation de dette. Chaque `#[allow(clippy::too_many_lines)]` est une promesse non tenue de revenir simplifier — à 616 LOC de fonctions trop longues, il est temps de tenir cette promesse.

Le code est prêt pour la publication en l'état — ces recommandations visent à faciliter la maintenance future, pas à corriger des bugs.
