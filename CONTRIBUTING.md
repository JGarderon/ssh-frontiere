# Contribuer à SSH Frontière

Merci de votre intérêt pour SSH Frontière ! Les contributions sont ouvertes à tous, y compris les contributions assistées ou générées par intelligence artificielle.

SSH Frontière est un **composant de sécurité**. La qualité du code et des tests est donc une priorité absolue.

## Licence

SSH Frontière est distribué sous la [Licence Publique de l'Union européenne (EUPL-1.2)](LICENSE.md).

**En soumettant une pull request, vous acceptez que votre contribution soit distribuée sous les termes de l'EUPL-1.2.** Chaque contributeur garantit que les droits d'auteur sur ses modifications lui appartiennent ou lui ont été donnés sous licence (article 6 de l'EUPL). C'est-à-dire que le projet est considéré par la communauté des développeurs comme *open-source*.

## Processus de contribution

### 1. Ouvrir une issue

Avant de commencer à coder, ouvrez une issue pour discuter du changement proposé. Cela permet de valider l'approche et d'éviter du travail inutile.

- **Bug** : décrivez le comportement observé vs attendu, la version, l'OS
- **Feature** : décrivez le cas d'usage, l'approche envisagée
- **Changement architectural** : une ADR (Architecture Decision Record) sera nécessaire — voir `docs/decisions/`

### 2. Fork et branche

```bash
git clone <votre-fork>
cd ssh-frontiere
git checkout -b feature/ma-contribution
```

### 3. Développer avec TDD

Le projet suit une méthodologie **TDD obligatoire** (RED-GREEN-RESOLUTION) :

1. **RED** : écrire le test qui échoue
2. **GREEN** : implémenter le code minimal pour que le test passe
3. **RESOLUTION** : refactorer si nécessaire

### 4. Vérifier localement

Avant d'ouvrir une PR, vérifiez que tout passe :

```bash
make lint     # cargo fmt --check + cargo clippy -- -D warnings
make test     # cargo nextest run (ou cargo test)
make audit    # cargo deny check + cargo audit
```

### 5. Ouvrir la pull request

- Décrivez le changement et son objectif
- Référencez l'issue associée
- Assurez-vous que la CI est verte

## Exigences de qualité

### Tests

Le code ajouté ou modifié doit être couvert à **90% minimum** par des tests unitaires et d'intégration. Pas de PR sans tests.

- Tests dans des fichiers séparés (`*_tests.rs` dans le même répertoire), pas inline
- Les tests E2E SSH (`tests/e2e-ssh/`) nécessitent Docker/Podman — ils sont exécutés par la CI, pas forcément par le contributeur

### Rust idiomatique

| Règle | Détail |
|-------|--------|
| Pas d'`unwrap()` | `#[deny(clippy::unwrap_used)]` — utiliser `expect()` avec justification `// INVARIANT:` ou `?` / `map_err()` |
| Pas d'`unsafe` | `#[deny(unsafe_code)]` — sauf justification documentée (ADR) |
| 800 LoC max | Par fichier source (hard limit) |
| 60 lignes max | Par fonction (hard limit) |
| Formatage | `cargo fmt` obligatoire |
| Lints | `cargo clippy -- -D warnings` (pedantic activé) |

### Dépendances

**Zéro dépendance non vitale.** SSH Frontière est un composant de sécurité : chaque dépendance transitive alourdit la surface d'attaque.

Avant de proposer une nouvelle dépendance :

1. Vérifiez que la stdlib Rust ne couvre pas le besoin
2. Évaluez avec la [matrice de dépendances](CLAUDE.md) (score minimum 3.5/5)
3. Documentez l'évaluation dans une note de recherche (`docs/searches/`)

Dépendances autorisées actuellement : `serde`, `serde_json`, `toml`.

## Conventions

### Commits

- Messages en **anglais**
- Format : `type(scope): description`
- Types : `feat`, `fix`, `refactor`, `test`, `docs`
- Exemples :
  - `feat(protocol): add TLS support`
  - `fix(dispatch): handle empty arguments`
  - `test(integration): add session timeout scenarios`

### Structure du code

- Fichiers source dans `src/`
- Fichiers de test dans `src/*_tests.rs` (même répertoire que le module testé)
- Tests d'intégration dans `tests/`
- Documentation dans `docs/`

## Contributions générées par IA

Les contributions assistées ou générées par intelligence artificielle sont les bienvenues. SSH Frontière est lui-même développé avec des agents IA (Claude Code).

Cependant :
- Le contributeur **reste responsable** de la qualité et de l'exactitude du code soumis
- Les mêmes exigences de tests et de qualité s'appliquent, quelle que soit l'origine du code
- Indiquez dans la PR si du code généré par IA a été utilisé (transparence)

## Sécurité

### Signaler une vulnérabilité

**Ne signalez pas les vulnérabilités via les issues publiques.**

Pour signaler une vulnérabilité de sécurité, contactez directement le mainteneur. Un processus de divulgation responsable sera suivi.

### Revue de sécurité

Les PR touchant les chemins critiques suivants feront l'objet d'une revue de sécurité renforcée :
- Authentification et autorisation (`protocol.rs`, `crypto.rs`)
- Parsing des commandes (`dispatch.rs`, `chain_parser.rs`)
- Exécution des commandes (`chain_exec.rs`)
- Gestion de la configuration (`config.rs`)

## Code de conduite

Ce projet adopte le [Contributor Covenant v2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/) comme code de conduite. En participant à ce projet, vous acceptez de respecter ses termes.

En résumé : soyez respectueux, inclusif et constructif. Tout comportement inacceptable peut être signalé au mainteneur du projet.

## Communication

- **Issues** : pour les bugs, features et discussions techniques
- **Pull requests** : pour les contributions de code
- **Discussions** : pour les questions générales (si disponible sur la forge)

## Pour commencer

Bonnes premières contributions ("good first issues") :
- Améliorer la documentation
- Ajouter des tests pour des cas limites
- Corriger des warnings clippy
- Améliorer les messages d'erreur
