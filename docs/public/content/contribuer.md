+++
title = "Contribuer"
description = "Comment contribuer à SSH-Frontière : processus, exigences, conventions"
date = 2026-03-24
weight = 6
+++

# Contribuer à SSH-Frontière

Les contributions sont les bienvenues, y compris les contributions assistées ou générées par intelligence artificielle. SSH-Frontière est lui-même développé avec des agents Claude Code.

## Avant de commencer

Ouvrez une **issue** pour discuter du changement proposé. Cela évite du travail inutile et permet de valider l'approche.

- **Bug** : décrivez le comportement observé vs attendu, la version, l'OS
- **Feature** : décrivez le cas d'usage et l'approche envisagée
- **Changement architectural** : une ADR sera nécessaire (voir `docs/decisions/`)

## Processus

```
1. Issue       → discuter du changement
2. Fork        → git checkout -b feature/ma-contribution
3. TDD         → RED (test qui échoue) → GREEN (code minimal) → refactorer
4. Vérifier    → make lint && make test && make audit
5. Pull request → décrire, référencer l'issue, CI verte
```

## Exigences de qualité

SSH-Frontière est un composant de sécurité. Les exigences sont strictes :

| Règle | Détail |
|-------|--------|
| Couverture tests | 90% minimum pour le code ajouté |
| Pas d'`unwrap()` | Utiliser `expect()` avec `// INVARIANT:` ou `?` / `map_err()` |
| Pas d'`unsafe` | Interdit par `#[deny(unsafe_code)]` |
| 800 lignes max | Par fichier source |
| 60 lignes max | Par fonction |
| Formatage | `cargo fmt` obligatoire |
| Lints | `cargo clippy -- -D warnings` (pedantic) |

### Dépendances

**Zéro dépendance non vitale.** Avant de proposer une nouvelle dépendance :

1. Vérifiez que la stdlib Rust ne couvre pas le besoin
2. Évaluez avec la matrice de dépendances (score minimum 3.5/5)
3. Documentez l'évaluation dans `docs/searches/`

Dépendances autorisées actuellement : `serde`, `serde_json`, `toml`.

## Conventions de commit

Messages en **anglais**, format `type(scope): description` :

- `feat(protocol): add TLS support`
- `fix(dispatch): handle empty arguments`
- `test(integration): add session timeout scenarios`
- `docs(references): update configuration guide`

Types : `feat`, `fix`, `refactor`, `test`, `docs`.

## Contributions IA

Les contributions générées par IA sont acceptées aux mêmes conditions que les contributions humaines :

- Le contributeur humain **reste responsable** de la qualité du code
- Mêmes exigences de tests et de lints
- Indiquez dans la PR si du code IA a été utilisé (transparence)

## Sécurité

### Signaler une vulnérabilité

**Ne signalez pas les vulnérabilités via les issues publiques.** Contactez directement le mainteneur pour une divulgation responsable.

### Revue renforcée

Les PR touchant ces fichiers font l'objet d'une revue de sécurité renforcée :

- `protocol.rs`, `crypto.rs` — authentification
- `dispatch.rs`, `chain_parser.rs`, `chain_exec.rs` — parsing et exécution des commandes
- `config.rs` — gestion de la configuration

## Bonnes premières contributions

- Améliorer la documentation
- Ajouter des tests pour des cas limites
- Corriger des warnings clippy
- Améliorer les messages d'erreur

## Licence

SSH-Frontière est distribué sous [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12). En soumettant une pull request, vous acceptez que votre contribution soit distribuée sous les termes de cette licence.

Pour les détails complets, consultez le fichier [CONTRIBUTING.md](https://github.com/nothus-forge/ssh-frontiere/blob/main/CONTRIBUTING.md) dans le dépôt.
