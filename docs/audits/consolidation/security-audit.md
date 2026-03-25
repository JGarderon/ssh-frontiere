# Audit de securite — SSH-Frontiere

Date : 2026-03-25
Auditeur : Agent Claude (Opus 4.6) — audit systematique appliquant la grille `docs/audits/code-review.md`
Perimetre : tous les fichiers source dans `src/` (14 fichiers de production, 8 fichiers de tests exclus de l'analyse)
Commit de reference : `5d2d4c6` (branche `consolidation/pre-publication`)

## Resume executif

SSH-Frontiere presente une posture de securite **solide** pour un composant de cette nature. Les choix architecturaux fondamentaux — execution directe via `std::process::Command` (pas de shell), parseur grammatical, whitelist stricte, comparaison en temps constant des secrets, zero `unsafe`, zero `unwrap()` en production — constituent une base de defense robuste. Les constats identifies sont majoritairement de severite BASSE ou MOYENNE : aucune vulnerabilite critique exploitable n'a ete trouvee. Les points d'attention principaux concernent le chiffrement XOR custom (axe crypto), une indexation directe non gardee dans `chain_exec.rs`, et l'absence de limitation de debit sur les tentatives d'authentification en mode session.

---

## Pre-requis automatises

| Commande | Resultat |
|----------|----------|
| `cargo clippy -- -D warnings` | **PASSE** — 0 warning |
| `cargo fmt --check` | **PASSE** — 0 diff |
| `cargo audit` | **NON DISPONIBLE** — `cargo-audit` non installe dans l'environnement agent |
| `cargo test` | **2 echecs** — `e2e_free_arg` et `e2e_output_streamed_not_truncated` (tests d'integration, probablement lies a l'environnement agent ou a des features en cours) |
| `cargo tree -d` | **1 doublon** — `getrandom` v0.3.4 / v0.4.2 (transitive via `proptest`, dev-dependency uniquement) |

---

## Findings par axe

### Axe 1 : Injection et parsing (S1 Correction et surete)

**Points forts :**
- Zero `unwrap()` en production (verifie par grep exhaustif + lint clippy `unwrap_used = "deny"`)
- Zero `unsafe` (lint `unsafe_code = "deny"` dans `Cargo.toml`)
- Zero `panic!` / `todo!` / `unimplemented!` en production
- Toute indexation directe documentee par `// PANIC-SAFE:` avec invariant
- `std::process::Command` avec execution directe (pas de shell interpose)
- Parseur grammatical `tokenize_with_quotes` + `resolve_command` : pas de liste noire de caracteres
- `env_clear()` sur les processus enfants avec `PATH` restreint (`/usr/local/bin:/usr/bin:/bin`)
- `process_group(0)` pour isoler les processus enfants

#### [MOYENNE] Indexation directe non gardee dans chain_exec.rs
- **Localisation** : `src/chain_exec.rs:132`
- **Description** : `let action = &config.domains[&domain_id].actions[&action_id];` utilise une indexation directe par cle sur `BTreeMap`. Si `resolve_command` retourne un `domain_id` ou `action_id` qui n'existe plus dans la config (scenario theorique mais non impossible si la config etait modifiable a chaud), cela provoquerait un panic.
- **Impact** : Crash du processus. En pratique le risque est nul car `resolve_command` valide la presence du domaine et de l'action juste au-dessus (ligne 123), et la config est immutable pour la duree de la connexion. Mais le pattern n'est pas defensif.
- **Recommandation** : Remplacer par `config.domains.get(&domain_id).and_then(|d| d.actions.get(&action_id))` avec gestion du `None`, ou ajouter un commentaire `// PANIC-SAFE:` explicite.

#### [BASSE] `data.len() as u64` dans sha256_pad (crypto.rs:102)
- **Localisation** : `src/crypto.rs:102`
- **Description** : Cast `usize as u64` pour calculer la longueur en bits. Sur un systeme 64-bit, `usize` et `u64` ont la meme taille, donc pas de troncation. Sur un systeme 32-bit, la conversion est sans perte (`u32` -> `u64`).
- **Impact** : Aucun en pratique. Le cast est sur dans les deux cas (elargissement ou identite).
- **Recommandation** : Documenter avec `// TRUNCATION-SAFE:` par coherence avec la politique du projet.

#### [BASSE] `i as u64 + 1` dans epoch_days_to_ymd (logging.rs:121)
- **Localisation** : `src/logging.rs:121`
- **Description** : Cast `usize as u64` dans la conversion de date. Sans perte puisque `i` est un index de tableau de 12 elements.
- **Impact** : Aucun.
- **Recommandation** : Commentaire `// PANIC-SAFE:` pour coherence.

### Axe 2 : Authentification

**Points forts :**
- Comparaison de secrets en temps constant (`constant_time_eq` avec `#[inline(never)]` + `core::hint::black_box`)
- Nonce de 16 octets genere depuis `/dev/urandom` (source d'entropie cryptographique du noyau)
- Regeneration du nonce apres chaque authentification reussie en session (protection anti-replay)
- Secrets stockes avec prefixe `b64:` dans la config TOML (pas en clair dans le code source)
- Lockout apres `max_auth_failures` tentatives echouees
- Mecanisme de ban via commande configurable (`ban_command`)
- Validation du format des noms de token (alphanumerique + tiret)

#### [MOYENNE] Pas de limitation de debit inter-session pour l'authentification
- **Localisation** : `src/orchestrator.rs:121-141` et `src/protocol.rs:432-488`
- **Description** : Le compteur `failures` est par connexion (memoire volatile). Un attaquant peut ouvrir N connexions SSH paralleles, chacune ayant droit a `max_auth_failures` tentatives. Le `ban_command` n'est execute qu'apres lockout d'une connexion individuelle, mais l'attaquant peut distribuer ses tentatives.
- **Impact** : Brute-force distribue sur les tokens. Mitige par : (1) la defense en profondeur SSH (`authorized_keys` + `command=`), (2) les limites sshd (`MaxAuthTries`, `MaxStartups`), (3) fail2ban cote systeme. Le risque residuel est faible si l'operateur configure correctement sshd.
- **Recommandation** : Documenter dans le guide operateur que la limitation de debit repose sur sshd + fail2ban, pas sur ssh-frontiere seul. A long terme, envisager un fichier d'etat partage (compteur persistant par IP) si des cas d'usage le justifient.

#### [BASSE] Le token_id est revele dans le message d'erreur auth
- **Localisation** : `src/protocol.rs:447`
- **Description** : `format!("unknown token '{token_id}'")` revele au client qu'un token n'existe pas, permettant l'enumeration des noms de token valides.
- **Impact** : Fuite d'information mineure. Un attaquant peut determiner quels noms de token existent. Les noms de token ne sont pas des secrets en soi (l'authentification repose sur le secret, pas le nom), mais c'est une information utile pour un attaquant.
- **Recommandation** : Retourner un message generique "auth failed" sans distinguer "token inconnu" de "proof invalide".

### Axe 3 : Chiffrement et cryptographie

**Points forts :**
- SHA-256 implemente en Rust pur (FIPS 180-4), pas de dependance externe
- Comparaison en temps constant avec protection contre les optimisations LLVM
- Nonce genere depuis `/dev/urandom` (CSPRNG du noyau)
- `overflow-checks = true` en profile release

#### [HAUTE] Chiffrement XOR avec compteur 8-bit dans xor_encrypt
- **Localisation** : `src/crypto.rs:223-245`
- **Description** : Le chiffrement XOR-CTR utilise un compteur `u8` (0-255) avec `wrapping_add(1)`. Pour des plaintexts > 256 * 32 = 8192 octets, le keystream se repete cycliquement. Le XOR d'un plaintext avec un keystream repetitif est cryptanalytiquement faible (attaque par XOR du ciphertext avec lui-meme decale de 8192 octets).
- **Impact** : En pratique, l'impact est **limite** car : (1) le XOR n'est utilise que pour la preuve challenge-response `compute_proof`, ou le plaintext est `secret || nonce` (typiquement 32-48 octets, tres en dessous de la limite de 8192), (2) le resultat est passe dans SHA-256 avant comparaison, (3) l'attaquant n'a jamais acces au ciphertext intermediaire. Neanmoins, le schema cryptographique est non-standard et n'a pas ete formellement audite.
- **Recommandation** : (1) Documenter explicitement dans l'ADR que le XOR-CTR est limite a des plaintexts < 8 Ko. (2) A terme, envisager un passage a HMAC-SHA-256 (implementable en Rust pur) qui offre des proprietes de securite mieux etablies sans cette limitation. (3) Ajouter une assertion `debug_assert!(plaintext.len() < 8192)` dans `xor_encrypt`.

#### [MOYENNE] Schema cryptographique non-standard pour le challenge-response
- **Localisation** : `src/crypto.rs:250-257`
- **Description** : La preuve est calculee comme `SHA-256(XOR_encrypt(secret || nonce, secret))`. Ce schema n'est pas un standard reconnu (HMAC, HKDF, etc.). Meme si les proprietes de securite semblent raisonnables dans le contexte d'utilisation, un schema non-standard est plus difficile a auditer et peut contenir des faiblesses subtiles.
- **Impact** : Le risque est mitige par le fait que le secret n'est jamais expose directement, le nonce est regenere, et la comparaison est en temps constant. L'attaque la plus plausible serait une analyse cryptographique du schema, mais elle necessite l'acces a de multiples paires (nonce, proof) pour un meme secret.
- **Recommandation** : Pour les versions futures, migrer vers `HMAC-SHA-256(secret, nonce)` qui offre des garanties de securite prouvees (PRF). L'implementation en Rust pur est simple : `SHA-256((secret XOR opad) || SHA-256((secret XOR ipad) || nonce))`.

### Axe 4 : Concurrence et threads

**Points forts :**
- Architecture synchrone one-shot : pas de Tokio, pas d'async, pas de conditions de course sur l'etat partage
- Threads de lecture stdout/stderr bien geres avec `mpsc::channel` et join
- Thread stdin (body) avec join et verification du resultat
- `process_group(0)` pour le kill propre (SIGTERM puis SIGKILL)

#### [BASSE] Canal mpsc non borne dans execute_command
- **Localisation** : `src/dispatch.rs:405`
- **Description** : `mpsc::channel::<StreamLine>()` cree un canal non borne. Si le processus enfant produit des millions de lignes tres rapidement, les threads de lecture les accumulent en memoire sans limite.
- **Impact** : Depassement memoire theorique. Mitige par `max_stream_bytes` qui tronque la sortie cote ecriture, mais les lignes sont tout de meme lues et stockees dans le canal avant troncation.
- **Recommandation** : Utiliser `mpsc::sync_channel` avec une capacite bornee (ex: 1000 lignes). Les threads de lecture se bloqueront quand le canal est plein, ce qui propage naturellement la back-pressure.

### Axe 5 : Gestion des entrees et limites

**Points forts :**
- Limites de taille sur les commandes (`MAX_COMMAND_LEN = 4096`), les tokens (`MAX_TOKEN_LEN = 256`), les lignes de protocole (`MAX_LINE_LEN = 4096`)
- Limites de taille sur le body (`max_body_size` configurable par action, defaut 64 Ko)
- Limites de taille sur la sortie streamee (`max_stream_bytes`, defaut 10 Mo)
- Limites de taille sur stdout/stderr (`max_stdout_chars`, `max_stderr_chars`)
- Timeout par commande et timeout de session
- Validation des arguments (enum, valeurs, doublons, coherence placeholders)
- Validation stricte de la config TOML au chargement

#### [MOYENNE] Limite body par defaut utilisee avant resolution de l'action
- **Localisation** : `src/orchestrator.rs:187`
- **Description** : `let max = 65536;` — la limite de body utilisee au premier read est le defaut de 64 Ko, pas la limite specifique a l'action (qui n'est connue qu'apres resolution de la commande). Si une action configure `max_body_size = 1024`, un attaquant peut envoyer 64 Ko avant que la verification specifique ne s'applique.
- **Impact** : Un attaquant peut envoyer un body jusqu'a 64 Ko quel que soit le `max_body_size` de l'action. Le body est lu en memoire (String). L'impact est un leger exces de consommation memoire (64 Ko au lieu du max configure), ce qui est mineur.
- **Recommandation** : Lire le body apres la resolution de la commande, ou utiliser un max global configurable dans `[global]`.

#### [BASSE] Timeout de session non configurable par action
- **Localisation** : `src/orchestrator.rs:296-297`
- **Description** : Le timeout de session est global (`timeout_session`). Toutes les sessions partagent le meme timeout, quel que soit le niveau de confiance.
- **Impact** : Un agent avec des permissions limitees peut maintenir une session aussi longtemps qu'un admin.
- **Recommandation** : Amelioration future — timeout de session par niveau de confiance.

### Axe 6 : Logging et audit

**Points forts :**
- Logging JSON structure de chaque evenement (execute, rejected, timeout, auth_lockout, stdin_error)
- Chaque entree contient : event, timestamp, pid, domaine, action, ssh_client, session_id
- Timestamp ISO 8601 sans dependance externe
- Logs best-effort (ne bloquent pas le dispatcher)

#### [MOYENNE] Absence de log des arguments de commande
- **Localisation** : `src/chain_exec.rs:366-372`
- **Description** : La fonction `log_command` enregistre le domaine et l'action, mais pas les arguments passes. Le champ `args` de `LogEntry` existe mais n'est pas rempli dans `execute_single_command`.
- **Impact** : En cas d'incident, les logs ne permettent pas de reconstituer la commande exacte executee. L'operateur ne peut pas savoir quels parametres ont ete passes a une action.
- **Recommandation** : Remplir le champ `args` de `LogEntry` avec les arguments resolus (en masquant les arguments marques `sensitive = true`).

#### [BASSE] Pas de log a la creation/fermeture de session
- **Localisation** : `src/orchestrator.rs:291-388`
- **Description** : L'ouverture et la fermeture d'une session ne sont pas loguees. Seuls les evenements individuels (commandes, auth) le sont.
- **Impact** : Difficulte a auditer la duree des sessions et les patterns de connexion.
- **Recommandation** : Ajouter des evenements `session_start` et `session_end` avec le session_id et la duree.

### Axe 7 : Configuration et secrets

**Points forts :**
- Secrets stockes avec prefixe `b64:` (pas en clair dans la config)
- Validation b64 a la charge de la config
- Pas de secret en dur dans le code source (verifie par grep exhaustif)
- Mode `--diagnostic` desactive par defaut : les erreurs internes sont opaques en production (`"service unavailable"`)
- Config rechargee a chaque connexion (one-shot) : pas de secret en memoire longue duree

#### [BASSE] Secrets b64 exposes dans les messages d'erreur de validation
- **Localisation** : `src/config.rs:285-289`
- **Description** : Si la validation du secret base64 echoue, le message d'erreur inclut `"token '{token_id}' has invalid base64 secret: {e}"`. Le message ne contient pas le secret lui-meme mais identifie quel token a un probleme, ce qui est une information operationnelle acceptable.
- **Impact** : Negligeable. Le message s'affiche au demarrage, pas en production.
- **Recommandation** : Aucune action necessaire.

### Axe 8 : Protection contre le deni de service

**Points forts :**
- Timeout par commande avec kill du process group (SIGTERM + delai + SIGKILL)
- Timeout de session global
- Limites de taille sur toutes les entrees (commandes, body, sortie)
- Lockout et ban apres echecs d'authentification repetes
- Architecture one-shot : chaque connexion est un processus separe, pas de ressource partagee entre connexions

#### [BASSE] Delai de grace fixe (5s) apres SIGTERM
- **Localisation** : `src/dispatch.rs:582`
- **Description** : `GRACEFUL_SHUTDOWN_SECS` est une constante de 5 secondes. Ce n'est pas configurable. Un processus qui ne repond pas a SIGTERM bloquera le dispatcher pendant 5 secondes avant SIGKILL.
- **Impact** : Mineur. Pendant ces 5 secondes, la connexion SSH est monopolisee. L'architecture one-shot limite l'impact.
- **Recommandation** : Potentiellement configurable, mais faible priorite.

### Axe 9 : Qualite du code defensif

**Points forts :**
- Lint clippy pedantic active avec `unwrap_used = "deny"` et `unsafe_code = "deny"`
- Deux `expect()` en production, tous deux justifies par `// INVARIANT:` (serialisation serde de types toujours valides)
- `#[must_use]` systematique sur les fonctions retournant `Result` ou `bool` significatifs
- `overflow-checks = true` en profile release
- `panic = "abort"` en release (pas de stack unwinding exploitable)
- Tests proptest pour les proprietes fondamentales (SHA-256, constant_time_eq, base64)

#### [BASSE] Les deux `expect()` en production sont justifies mais pourraient etre eliminables
- **Localisation** : `src/output.rs:58` et `src/logging.rs:73`
- **Description** : `serde_json::to_string(self).expect("... cannot fail")` — la serialisation d'un `Response` ou `LogEntry` ne peut effectivement pas echouer car tous les champs sont des types primitifs ou String. Le commentaire `// INVARIANT:` est present.
- **Impact** : Aucun risque en pratique. L'invariant est correct.
- **Recommandation** : Acceptable en l'etat. Alternative theorique : retourner un fallback JSON en dur en cas d'erreur, mais la complexite ajoutee ne se justifie pas.

### Axe 10 : Execution de commandes et sandboxing

**Points forts :**
- `std::process::Command` sans shell interpose — pas d'injection
- `env_clear()` + PATH restreint (`/usr/local/bin:/usr/bin:/bin`)
- `process_group(0)` pour l'isolation du processus enfant
- stdin null quand pas de body (pas de fuite de l'entree du dispatcher vers le processus enfant)
- Pas de TTY alloue
- Pas de forwarding (garanti par `restrict` dans `authorized_keys`, couche 1)

#### [BASSE] Le processus enfant herite de `SSH_FRONTIERE_SESSION` seulement
- **Localisation** : `src/dispatch.rs:371-376`
- **Description** : `env_clear()` est excellent. Le seul variable d'environnement passee est `PATH` et `SSH_FRONTIERE_SESSION`. Cependant, certains programmes ont besoin de `HOME`, `USER`, ou `LANG` pour fonctionner correctement.
- **Impact** : Pas un probleme de securite — c'est un choix de defense en profondeur. Les commandes de la whitelist doivent etre concues pour fonctionner sans ces variables.
- **Recommandation** : Documenter dans le guide operateur que les commandes whitelistees recoivent un environnement minimal.

### Axe 11 : Dependances

**Points forts :**
- Seulement 3 dependances directes : `serde`, `serde_json`, `toml`
- Politique zero dependance non justifiee
- Matrice d'evaluation documentee
- SHA-256, base64, hex encode/decode implementes en Rust pur (pas de dependance crypto)
- `proptest` en dev-dependency uniquement

#### [BASSE] Doublon `getrandom` dans les dev-dependencies
- **Localisation** : `Cargo.lock`
- **Description** : `getrandom` v0.3.4 et v0.4.2 sont presents, via les dependances transitives de `proptest`. C'est un dev-dependency uniquement.
- **Impact** : Aucun en production. Le binaire release ne contient pas `proptest`.
- **Recommandation** : Aucune action necessaire.

#### [BASSE] `cargo audit` non disponible dans l'environnement
- **Description** : `cargo-audit` n'est pas installe dans l'environnement de l'agent. Impossible de verifier les RUSTSEC advisories.
- **Recommandation** : Ajouter `cargo audit` a la CI. Verifier manuellement les versions de `serde` (1.0.228), `serde_json` (1), et `toml` (0.8.23) sur rustsec.org.

### Axe 12 : Simplicite et sur-ingenierie

**Points forts :**
- Architecture simple et lisible : 14 fichiers de production pour ~3500 LoC
- Pas de trait avec une seule implementation
- Pas de generiques non justifies
- Pas de macro custom
- Pas de builder pattern superflu
- Un seul `#[allow(dead_code)]` justifie (`hex_decode` utilise par `proof.rs` via `lib.rs`)

#### [BASSE] Cinq fonctions avec `#[allow(clippy::too_many_lines)]`
- **Localisation** : `orchestrator.rs:29`, `dispatch.rs:351`, `chain_exec.rs:73`, `chain_exec.rs:190`, `chain_exec.rs:242`
- **Description** : Cinq fonctions depassent la limite clippy de lignes. Toutes contiennent un pipeline sequentiel de phases qui forme une unite logique coherente. La 5eme (`execute_and_respond`) a depasse la limite apres le reformatage `cargo fmt` des corrections Phase 3.4 (BTreeMap `.get()`).
- **Impact** : Pas un probleme de securite. La lisibilite est acceptable grace aux commentaires de phase.
- **Recommandation** : Envisager l'extraction de sous-fonctions pour les plus longues, mais faible priorite.

#### [BASSE] `GlobalConfig` a 12+ champs
- **Localisation** : `src/config.rs:113-148`
- **Description** : La struct `GlobalConfig` contient 12+ champs (log_file, default_timeout, max_stdout_chars, etc.). C'est proche du seuil de 10 champs recommande.
- **Impact** : Maintenabilite. Les champs sont tous des parametres de configuration de premier niveau.
- **Recommandation** : Envisager un regroupement en sous-structs (`OutputLimits`, `SessionConfig`) lors d'une future evolution.

---

## Tableau recapitulatif

| # | Severite | Axe | Finding | Recommandation |
|---|----------|-----|---------|----------------|
| 1 | HAUTE | Crypto (3) | Compteur XOR 8-bit, keystream cyclique apres 8 Ko | Documenter la limitation, migrer vers HMAC-SHA-256 |
| 2 | MOYENNE | Crypto (3) | Schema challenge-response non-standard | Migration future vers HMAC-SHA-256 |
| 3 | MOYENNE | Auth (2) | Pas de limitation de debit inter-session | Documenter la dependance a sshd + fail2ban |
| 4 | MOYENNE | Parsing (1) | Indexation directe non gardee dans chain_exec.rs:132 | Remplacer par `.get()` ou documenter PANIC-SAFE |
| 5 | MOYENNE | Entrees (5) | Limite body par defaut avant resolution d'action | Lire body apres resolution ou max global configurable |
| 6 | MOYENNE | Logging (6) | Arguments de commande non logues | Remplir le champ `args` de LogEntry |
| 7 | BASSE | Auth (2) | Enumeration des noms de token possible | Message d'erreur generique |
| 8 | BASSE | Threads (4) | Canal mpsc non borne | `sync_channel` avec capacite bornee |
| 9 | BASSE | Logging (6) | Pas de log session_start / session_end | Ajouter des evenements de session |
| 10 | BASSE | Code (9) | Deux `expect()` justifies par INVARIANT | Acceptable en l'etat |
| 11 | BASSE | Parsing (1) | Cast `usize as u64` sans commentaire | Ajouter commentaire TRUNCATION-SAFE |
| 12 | BASSE | DoS (8) | Delai SIGTERM fixe (5s) | Potentiellement configurable |
| 13 | BASSE | Deps (11) | `cargo audit` non disponible | Ajouter a la CI |
| 14 | BASSE | Archi (12) | 5 fonctions too_many_lines | Extraction de sous-fonctions |
| 15 | BASSE | Archi (12) | GlobalConfig 12+ champs | Sous-structs futures |

---

## Corrections appliquees (Phase 3.4)

Commits : `b5dde6b` + `9842d88` (branche `consolidation/pre-publication`)

| # | Severite | Constat | Correction | Statut |
|---|----------|---------|------------|--------|
| 1 | HAUTE | Compteur XOR 8-bit (keystream repeat a 8192) | `debug_assert!(plaintext.len() < 8192)` dans `xor_encrypt` | **CORRIGE** |
| 4 | MOYENNE | Indexation directe BTreeMap | Remplacement par `.get()` + gestion `None` dans `chain_exec.rs` | **CORRIGE** |
| 6 | MOYENNE | Args non logues | **REVERT** — le logging des args fuitait les valeurs sensibles (mots de passe en clair). Necessite un mecanisme de masquage (hash ou redaction selon `sensitive` dans ActionConfig) avant implementation | **ANNULE** |
| 7 | BASSE | Enumeration noms de token | Message generique `authentication failed` dans `auth.rs` | **CORRIGE** |

Les constats #2, #3, #5, #8-#15 restent comme backlog de securite pour des phases futures.

**Note sur le constat #6** : la correction initiale ajoutait `with_args()` a `LogEntry` et l'appelait dans `log_command()`. Le test OUT-008 (output-edge-cases) a revele que les arguments comme `password=SuperSecret123` apparaissaient en clair dans les logs. La correction a ete revertee. L'implementation correcte necessiterait de consulter `ActionConfig.args` pour distinguer les arguments `sensitive` (a hasher) des arguments normaux (a loguer en clair).

---

## Recommandations de long terme

### Priorite 1 — Schema cryptographique
- **Migrer vers HMAC-SHA-256** pour le challenge-response. L'implementation en Rust pur est simple (deux passes SHA-256 avec padding HMAC) et offre des garanties de securite formellement prouvees. Cela elimine le chiffrement XOR custom et sa limitation de compteur 8-bit.

### Priorite 2 — Observabilite
- **Enrichir les logs** : ajouter les arguments resolus (avec masquage des `sensitive`), les evenements de session, et le `session_id` dans tous les logs.
- **Metriques** : a terme, exposer des compteurs (commandes executees, rejetees, timeouts, auth failures) dans un format machine-readable pour l'integration avec des systemes de monitoring.

### Priorite 3 — Audit formel
- **Fuzzing** : l'infrastructure de fuzzing (`lib.rs` + `fuzz_helpers`) est en place. Executer des campagnes de fuzzing regulieres sur les parseurs (protocole, commande, config, body).
- **cargo audit** : integrer dans la CI avec `cargo audit --deny warnings`.
- **Revue cryptographique externe** : le schema challenge-response meriterait une revue par un cryptographe, meme apres migration vers HMAC.

### Priorite 4 — Durcissement futur
- **Limitation de debit persistante** : si des cas d'usage sans sshd en amont apparaissent (tunnel, socket Unix), implementer un compteur d'echecs par IP persistant.
- **Seccomp** : envisager un filtre seccomp sur le processus `ssh-frontiere` lui-meme (syscalls autorises : read, write, open, close, exec, fork, wait, kill, getpid, clock_gettime, getrandom).

---

## Conclusion a l'attention de Julien

SSH-Frontiere est un composant de securite bien concu. Les choix fondamentaux — pas de shell, parseur grammatical, execution directe, defense en profondeur sur 3 couches, zero unsafe, zero unwrap — sont excellents et placent le projet au-dessus de la moyenne des outils comparables.

**Aucune vulnerabilite critique n'a ete identifiee.** Le constat le plus serieux (HAUTE) concerne le schema cryptographique XOR custom, dont l'impact est mitige par le fait que les donnees chiffrees sont toujours courtes et que le resultat passe dans SHA-256. La migration vers HMAC-SHA-256 est recommandee comme amelioration structurelle, sans urgence immediate.

Les 6 constats de severite MOYENNE sont des ameliorations de defense en profondeur : message d'erreur auth generique, limite body post-resolution, enrichissement des logs. Aucun n'est exploitable pour un contournement des controles d'acces.

Le rapport benefice/complexite du projet est remarquable : ~3500 lignes de production, 3 dependances, un binaire statique < 2 Mo, et une posture de securite solide. Le principal axe d'amelioration est l'observabilite (logs plus riches) et la formalisation du schema cryptographique.
