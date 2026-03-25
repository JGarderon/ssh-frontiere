# ADR 0005 — Implémentation SHA-256 : code maison vs crate sha2

**Date** : 2026-03-15
**Statut** : Acceptée
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : TODO-002, CLAUDE.md (politique de dépendances), ADR 0002 (secrets niveau 1)
**Phase** : 2 — Intégration serveur

---

## Contexte

L'implémentation SHA-256 actuelle (~80 LoC dans `logging.rs`) est en Rust pur, sans dépendance externe. Elle est utilisée **uniquement** pour le masquage des arguments sensibles dans les logs (ADR 0002, secrets niveau 1).

Usage concret : quand `mask_sensitive = true` dans la config et qu'un argument est marqué `sensitive = true`, sa valeur est remplacée par son empreinte SHA-256 dans le fichier de log JSON. L'empreinte n'est **jamais** utilisée pour de la vérification d'intégrité ni de l'authentification.

L'implémentation est testée contre 3 vecteurs NIST (FIPS 180-4) : chaîne vide, "abc", message de 448 bits.

La question : faut-il remplacer cette implémentation par la crate `sha2` du projet RustCrypto ?

---

## Évaluation de la crate `sha2` (matrice CLAUDE.md)

Données factuelles vérifiées sur crates.io et GitHub le 2026-03-15 :

| Critère | Données factuelles | Note /5 | Poids | Score |
|---------|-------------------|---------|-------|-------|
| **Licence** | MIT OR Apache-2.0 | ✅ | Éliminatoire | PASS |
| **Origine et gouvernance** | RustCrypto, organisation communautaire mondiale, pas d'affiliation étatique identifiable | 4 | ×3 | 12 |
| **Communauté** | 511M téléchargements totaux, 93M récents, 2.2k stars GitHub, 85 contributeurs | 5 | ×2 | 10 |
| **Fréquence MAJ** | Dernière release : fév. 2026, créée en 2016, active (v0.10.9 stable + v0.11 RC) | 4 | ×2 | 8 |
| **Taille** | Pure Rust, mais ajoute 3 dépendances transitives requises | 3 | ×3 | 9 |
| **Dépendances transitives** | `digest`, `cfg-if`, `cpufeatures` (+ `sha2-asm` optionnel) | 3 | ×3 | 9 |
| **Fonctionnalités** | SHA-224/256/384/512, no_std, traits standardisés | 5 | ×2 | 10 |
| **Non-enfermement** | Trait `Digest` standard, remplacement trivial | 5 | ×1 | 5 |

**Score pondéré** : (12 + 10 + 8 + 9 + 9 + 10 + 5) / (3 + 2 + 2 + 3 + 3 + 2 + 1) = **63 / 16 = 3.94/5**

Le score est au-dessus du seuil d'adoption (3.5/5). La crate `sha2` est **éligible** selon la matrice.

---

## Options

### Option A — Garder l'implémentation maison

**Pour** :
- Zéro dépendance supplémentaire (surface d'attaque minimale, binaire plus léger)
- L'usage est limité au masquage de logs, pas de la cryptographie critique
- L'algorithme SHA-256 est bien documenté (FIPS 180-4), l'implémentation est directe
- 3 vecteurs NIST validés, code lisible (~80 LoC)
- Cohérent avec la politique "zéro dépendance non vitale" de CLAUDE.md

**Contre** :
- Implémentation non auditée formellement
- 3 vecteurs de test seulement (ne couvre pas les cas limites cryptographiques)
- Risque théorique de bug subtil (timing, cas limites de padding)

### Option B — Migrer vers `sha2` (RustCrypto)

**Pour** :
- Implémentation largement auditée et testée (511M téléchargements)
- Couverture de test exhaustive (vecteurs NIST complets + edge cases)
- Optimisations hardware (SIMD via `cpufeatures`)
- Maintenance communautaire active

**Contre** :
- +3 dépendances transitives requises (`digest`, `cfg-if`, `cpufeatures`)
- Augmentation de la surface d'attaque (supply chain)
- Augmentation de la taille du binaire
- Surdimensionné pour l'usage (masquage de logs)

---

## Décision

**Option A — Garder l'implémentation maison.**

### Justification

1. **Proportionnalité** : SHA-256 est utilisé ici pour du masquage de logs, pas pour de la vérification d'intégrité ni de l'authentification. Un bug subtil dans l'implémentation (timing side-channel, edge case de padding) n'aurait **aucun impact de sécurité** — le pire cas serait un hash incorrect dans un fichier de log.

2. **Politique de dépendances** : CLAUDE.md impose "zéro dépendance non vitale". Ajouter 4 crates (sha2 + 3 transitives) pour du masquage de logs viole l'esprit de cette règle. Les 3 dépendances actuelles (serde, serde_json, toml) sont justifiées par des besoins structurels (sérialisation JSON, parsing TOML). SHA-256 pour le masquage ne l'est pas.

3. **Surface d'attaque** : SSH Frontière est un composant de sécurité. Chaque dépendance transitive est un vecteur d'attaque supply chain potentiel. 0 dépendance supplémentaire > 4 dépendances supplémentaires.

4. **Binaire** : la cible est < 2 Mo en release musl. Chaque crate ajoutée alourdit le binaire.

### Mesure compensatoire

Pour renforcer la confiance dans l'implémentation maison, **ajouter des vecteurs de test NIST supplémentaires** :
- Message de 1 000 000 fois le caractère "a" (vecteur NIST classique)
- Messages de longueur exacte 55, 56, 64 octets (limites de padding)

Ces vecteurs couvrent les cas limites les plus courants de SHA-256 et sont vérifiables contre les publications NIST.

### Condition de révision

Si l'usage de SHA-256 évolue vers de la **vérification d'intégrité** ou de l'**authentification** (Phase 3 : token RBAC), cette décision doit être révisée. La crate `sha2` serait alors justifiée par un besoin de sécurité critique.

---

## Conséquences

### Positives
- Zéro nouvelle dépendance
- Taille binaire inchangée
- Surface d'attaque supply chain inchangée
- Vecteurs de test supplémentaires renforcent la confiance

### Négatives
- L'implémentation reste non auditée formellement (risque accepté car usage non critique)
- Si le besoin évolue (Phase 3), il faudra réévaluer

---

## Attribution

- **Julien (BO)** : politique zéro dépendance, validation de l'usage non critique
- **Claude (PM/Tech Lead)** : évaluation matrice, analyse risque/bénéfice, mesure compensatoire
- **Agents Claude Code** : collecte données factuelles crates.io/GitHub, rédaction ADR
