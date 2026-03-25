# Couverture de tests — Baseline pré-consolidation

**Date** : 2026-03-25
**Outil** : cargo-tarpaulin 0.35.2
**Commande** : `cargo tarpaulin --workspace --out stdout`
**Couverture globale** : 70.71% (1014/1434 lignes)

---

## Couverture par module

| Module | Couvert | Total | Couverture | Statut |
|--------|---------|-------|------------|--------|
| `src/output.rs` | 13 | 13 | **100%** | OK |
| `src/crypto.rs` | 151 | 151 | **100%** | OK |
| `src/logging.rs` | 59 | 62 | **95.2%** | OK |
| `src/config.rs` | 98 | 103 | **95.1%** | OK |
| `src/chain_parser.rs` | 151 | 160 | **94.4%** | OK |
| `src/protocol.rs` | 239 | 254 | **94.1%** | OK |
| `src/discovery.rs` | 54 | 59 | **91.5%** | OK |
| `src/dispatch.rs` | 196 | 226 | **86.7%** | À compléter |
| `src/chain_exec.rs` | 53 | 158 | **33.5%** | Critique |
| `src/bin/proof.rs` | 0 | 27 | **0%** | Binaire CLI |
| `src/lib.rs` | 0 | 7 | **0%** | Ré-exports |
| `src/main.rs` | 0 | 18 | **0%** | Point d'entrée |
| `src/orchestrator.rs` | 0 | 196 | **0%** | Intégration only |

## Analyse des modules sous 90%

### `chain_exec.rs` — 33.5% (critique)

Module d'exécution des chaînes de commandes. Fortement sous-couvert. Contient la logique d'exécution avec timeouts, signaux, gestion des process groups. Les tests unitaires ne couvrent que les chemins simples ; les chemins d'erreur (timeout, kill, signal handling) sont peu testés.

### `dispatch.rs` — 86.7%

Proche de la cible. Manquent principalement des branches de validation d'arguments et certains chemins d'erreur RBAC.

### `orchestrator.rs` — 0%

Ce module est exercé exclusivement via les tests d'intégration qui lancent le binaire en subprocess. `tarpaulin` ne peut pas instrumenter ces processus fils. La couverture réelle est bien supérieure à 0% — elle est captée par les 50 tests d'intégration + 72 scénarios E2E SSH Docker.

**Justification** : la nature même du module (orchestration I/O stdin/stdout, bannière, lecture d'en-têtes) le rend difficile à tester unitairement sans mocking lourd. Les tests d'intégration (subprocess) sont le bon mécanisme de vérification.

### `main.rs` — 0%

Point d'entrée. Parsing des arguments, fail-fast si config manquante. Couvert par les tests d'intégration (subprocess). Code minimal (18 lignes).

### `lib.rs` — 0%

Ré-exports pour le binaire `proof`. 7 lignes. Couvert indirectement.

### `bin/proof.rs` — 0%

Binaire CLI utilitaire pour calculer les proofs d'authentification. Nécessiterait des tests subprocess dédiés. 27 lignes — impact limité.

## Cible consolidation

Objectif ADR 0013 : **≥ 90% par module** (hors justifications).

Modules nécessitant des tests supplémentaires :
1. **`chain_exec.rs`** — priorité haute, +95 lignes à couvrir
2. **`dispatch.rs`** — priorité moyenne, +30 lignes à couvrir
3. **`proof.rs`** — priorité basse, tests subprocess simples

Modules justifiés sous 90% :
- `orchestrator.rs` — couvert par intégration subprocess
- `main.rs` — couvert par intégration subprocess
- `lib.rs` — ré-exports triviaux
