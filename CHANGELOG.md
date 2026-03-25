# Changelog

Toutes les modifications notables de ce projet sont documentées dans ce fichier.

Le format est basé sur [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/),
et ce projet adhère au [Semantic Versioning](https://semver.org/lang/fr/).

## [Unreleased]

## [3.1.0] — 2026-03-25 (Consolidation pré-publication)

### Ajouté
- **Consolidation 3 phases** (ADR 0013) : tests ≥ 90%, simplification du code, audit de sécurité
- 99 scénarios E2E comportementaux en Python (`tests/scenarios/`) — 8 familles (injection, auth, RBAC, protocole, timeout, sessions, config, output)
- Modules `src/executor.rs` et `src/auth.rs` extraits de dispatch.rs et protocol.rs
- Grille d'analyse de sécurité (`docs/audits/grids/security-review.md`) — 12 axes
- Audit de sécurité : 0 CRITIQUE, 1 HAUTE corrigée, 5 MOYENNE, 9 BASSE
- Audit de simplification du code
- Note de recherche `docs/searches/008-securite-login-shell-rust.md`
- Site vitrine Zola en 7 langues (FR, EN, DE, ES, PT, JA, KO) avec sélecteur de langue
- README multilingues (`README_fr.md`, `README_en.md`, `README_de.md`, `README_es.md`, `README_pt.md`, `README_ja.md`, `README_ko.md`)
- GitHub Actions CI (`cargo test`, `clippy`, `fmt`) et release automatique (binaire musl statique + SHA256SUMS)
- `LICENSE.md` — texte intégral de l'EUPL-1.2 en français
- `CONTRIBUTING.md` — guide de contribution
- `CHANGELOG.md` — historique des versions

### Modifié
- Messages utilisateur traduits du français vers l'anglais (help, protocole, erreurs)
- Couverture de tests portée de 70% à ≥ 90% sur tous les modules
- Fonctions longues découpées (orchestrator::run, dispatch::execute_command, chain_exec)
- `README.md` transformé en hub multilingue
- Licence passée de « propriétaire » à **EUPL-1.2**
- `Cargo.toml` : version 3.0.0, ajout `license`, `keywords`, `categories`

### Corrigé
- Race condition dans les tests d'intégration (chemins temporaires partagés)
- Assert sur la longueur XOR keystream (HAUTE sécurité)
- Indexation directe BTreeMap remplacée par `.get()` (MOYENNE sécurité)
- Énumération des noms de token neutralisée (BASSE sécurité)

### Sécurité
- `debug_assert!(plaintext.len() < 8192)` dans `xor_encrypt` — borne le keystream XOR
- Message d'erreur auth générique — empêche l'énumération de tokens
- Logging des arguments revert — empêche la fuite de secrets dans les logs

## [3.0.0] — 2026-03-20 (Phase 9)

### Ajouté
- Protocole body (`+body`, `+body size=N`, `+body stop="X"`) — ADR 0012
- Arguments libres (`free = true` dans la config)
- `max_body_size` configurable par action (défaut 64 Ko)
- Code de sortie 133 (stdin fermé lors de l'envoi du body)
- Capability `body` dans la bannière serveur
- Champs `max_body_size` et `free` dans la découverte (`help`)
- Fuzzing : harnesses body, transpose_command, config TOML
- Propriétés proptest pour le body
- Scénarios E2E body (RED, en attente Docker)

### Corrigé
- `transpose_command` avec arguments contenant des espaces
- Nettoyage champs morts `GlobalConfig` (M-11, S-03)

## [2.2.0] — 2026-03-19 (Phase 8)

### Ajouté
- Types d'erreur structurés : `DispatchError`, `ConfigError`, `ChainError`
- `deny.toml` pour vérification licences et advisories (`cargo-deny`)
- Clippy pedantic activé (`warn` → `deny` pour les lints critiques)
- `unsafe_code = "deny"` dans Cargo.toml
- Renforcement `constant_time_eq` : `#[inline(never)]` + `black_box`
- `#[must_use]` sur fonctions pub retournant Result/bool
- cargo-fuzz : 6 harnesses de fuzzing
- proptest : 5 propriétés
- cargo-vet : chaîne d'approvisionnement vérifiée
- ADR 0012 (texte libre et ressources) rédigée

### Modifié
- Découpage `chain.rs` → `chain_parser.rs` + `chain_exec.rs`
- Découpage `dispatch.rs` → `dispatch.rs` + `discovery.rs`
- `pub` → `pub(crate)` pour les items internes (~30 items)
- `clone()` → `mem::take` (2 sites)
- `tokens.remove(0)` → drain/index
- Annotation `// PANIC-SAFE:` sur 15+ sites d'indexation directe
- `SessionContext` struct (refactoring 8 params → struct)
- Factorisation `ActionConfig::is_visible_to()` (5 duplications supprimées)
- Suppression ~80 lignes de code mort (v1/v2)

### Corrigé
- 22× `expect_err` → `unwrap_err` dans les tests (clippy)
- Durcissement `execute_ban_command` (sanitization IP)
- Match exhaustif dans protocol.rs (wildcard → exhaustif)
- Fallback UUID : warning loggé

## [2.1.0] — 2026-03-18 (Phase 7)

### Ajouté
- Guide complet de rédaction config.toml (`docs/references/guide-configuration.md`)
- `--check-config` : validation de la configuration en mode dry-run
- `help` sans préfixe `$` → texte humain lisible
- Bannière : `#> type "help" for available commands`

### Modifié
- Ligne vide optionnelle entre entêtes et commande (plus permissif)
- ADR 0006 mise à jour (2 addendums)

## [2.0.0] — 2026-03-18 (Phases 5 + 5.5)

### Ajouté
- Tags de visibilité et filtrage horizontal (ADR 0008) — Phase 5
- Champ `tags` sur `ActionConfig` et `TokenConfig`
- `check_tags()` : intersection tags identité × action
- Filtrage `help`/`list` par tags (en plus du niveau)
- Nonce optionnel (ADR 0010) — Phase 5.5
- `challenge_nonce` config (bool, défaut false, mode simple SHA-256)
- Binaire `ssh-frontiere-proof` : mode sans nonce (`--secret` seul)
- Arguments nommés `key=value` (ADR 0009) — Phase 5.5
- `args` de `Vec<ArgDef>` à `BTreeMap<String, ArgDef>`
- Champ `default` sur `ArgDef` (valeurs par défaut)
- Extraction `main.rs` → `orchestrator.rs` pour testabilité

### Modifié
- Parsing exclusivement nommé (`key=value`), positionnel supprimé
- Tags effectifs proviennent exclusivement des tokens RBAC
- Fusion tags union cumulative en session
- Logging : champs `effective_tags` et `action_tags` dans `LogEntry`

### Supprimé
- Champ `name` de `ArgDef` (redondant avec la clé BTreeMap)
- Support des arguments positionnels

## [1.0.0] — 2026-03-16

### Ajouté
- Environnement de test E2E SSH Docker (56 scénarios)
- Binaire `ssh-frontiere-proof` pour les tests E2E
- Documentation limites crypto et rate limiting
- Guide opérateur

### Modifié
- Suppression de FORBIDDEN_CHARS, modèle parseur grammatical
- Récursion → boucle itérative (protocol.rs)
- Régénération du nonce après +auth réussi (anti-replay)
- Bouchon bash mis à jour pour le protocole Phase 3

### Corrigé
- 13 TODOs fermés

## [0.4.0] — 2026-03-15 (Phase 3)

### Ajouté
- Protocole d'entêtes unifié (ADR 0006) : 4 préfixes (+, #, $, >)
- Authentification RBAC challenge-response (SHA-256, XOR, base64)
- Mode session opt-in (+session keepalive)
- Protection auth (max_auth_failures, ban optionnel)
- Module crypto.rs : SHA-256, base64, nonce, XOR cipher
- Module protocol.rs : parseur, bannière, entêtes, auth, session

## [0.3.0] — 2026-03-15 (Phase 2.5)

### Modifié
- SHA-256 NIST FIPS 180-4 : 4 vecteurs de test
- HashMap → BTreeMap pour découverte déterministe
- Timeout gracieux SIGTERM → SIGKILL
- Tests de conformité partagés stub/binaire

## [0.2.0] — 2026-03-14 (Phase 2)

### Ajouté
- Configuration production (sudoers, authorized_keys, deploy scripts)
- Bouchon bash (fallback)
- Scripts opérations (backup, deploy, healthcheck)
- ADR 0005 : SHA-256 maison conservé

## [0.1.0] — 2026-03-13 (Phase 1)

### Ajouté
- Dispatcher fonctionnel (7 concepts alignement 001)
- Config TOML hiérarchique (domaines, actions, arguments)
- RBAC read/ops/admin, exécution via std::process::Command
- Retour JSON 4 champs, codes de sortie (0, 128-131)
- Logging JSON structuré avec masquage SHA-256
- Découverte LLM-compatible (help, list)
