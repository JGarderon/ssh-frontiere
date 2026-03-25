# ADR 0013 — Stratégie de consolidation pré-publication

**Date** : 2026-03-25
**Statut** : Accepted
**Participants** : Julien (BO), Claude superviseur (PM/Tech Lead)

---

## Contexte et problème

SSH-Frontière est prêt fonctionnellement (v3 en production, 399 tests, site vitrine rédigé, licence EUPL-1.2 choisie). La publication open-source est la prochaine étape.

La qualité perçue lors de la première diffusion conditionne l'adoption. Un code « pas terrible » avec des retours négatifs initiaux serait irrécupérable — le produit ne s'en relèverait pas. Les tests unitaires seuls ne suffisent pas à garantir la qualité (leçon CHAT-009b sur agent-alex : 4 corrections avec tests verts, bug toujours présent).

Il faut donc une stratégie de consolidation rigoureuse avant publication, qui servira ensuite de **politique permanente** pour les futures versions et PR.

## Décision

### Trois phases séquentielles obligatoires

La consolidation suit trois phases dans un ordre strict. Chaque phase doit être validée avant de passer à la suivante. Si une phase échoue après correction, on relance les phases en aval (voire tout depuis le début si la correction est structurelle).

#### Phase 1 — Tests et couverture

**Objectif** : verrouiller le comportement du produit. Les tests « figent » le fonctionnement.

1. **Couverture ≥ 90%** sur tous les modules. Si un module ne l'atteint pas, le justifier dans une note d'audit des tests (`docs/audits/test-coverage.md`).

2. **Scénarios E2E comportementaux** (nouvelle famille de tests) :
   - Écrits dans `docs/scenarios/`, un fichier MD par grande famille de scénarios
   - Rédigés **sans relecture du code** — uniquement à partir de la documentation et des cas d'usage attendus
   - Simulés via scripts Python qui lancent le binaire directement (`subprocess`), sans SSH réel
   - Couvrent massivement les pannes, mauvais comportements, entrées invalides, timeouts, configurations corrompues
   - Les commandes réellement exécutées sont des scripts bouchons qui répondent en erreur ou succès selon le scénario

3. **Compléter les tests E2E Docker existants** (72 scénarios) si des manques sont identifiés lors de la rédaction des scénarios.

4. **Bugs détectés** → corrigés immédiatement en TDD RED-GREEN dans la foulée.

#### Phase 2 — Simplification du code

**Objectif** : réduire la surface de complexité. Éviter la complexité gratuite.

1. **Audit de simplification** (`docs/audits/simplification.md`) couvrant :
   - Toutes les structures Rust du projet
   - Toutes les fonctions > 35 LOC
   - Les modules à faible cohésion (fichiers « fourre-tout » où les fonctions ne vont pas ensemble)
   - Pour chaque point : est-ce bien pensé ? Peut-on faire plus propre, avec moins de code, de manière plus efficace ?

2. **Livrables de l'audit** :
   - Liste des points à corriger (classés par priorité)
   - Recommandations de long terme pour la construction du code
   - Conclusion à l'attention de Julien

3. **Relecture agressive** de la note d'audit par le superviseur avant exécution.

4. **Corrections** via le cycle Generator/Evaluator. Les tests de la phase 1 servent de filet anti-régression.

#### Phase 3 — Audit de sécurité

**Objectif** : détecter et corriger toutes les vulnérabilités. La sécurité est jugée en dernier car elle s'applique au code simplifié et testé.

1. **Note de recherche** (`docs/searches/`) sur les référentiels de sécurité applicables :
   - OWASP pour CLI/outils système
   - Patterns spécifiques Rust (unsafe, panics, side channels, timing attacks)
   - Sécurité des dépendances (supply chain)
   - Guides de durcissement pour login shells et composants SSH

2. **Grille d'analyse de sécurité** (`docs/audits/code-review-security.md`) déduite de la note de recherche. Couvre :
   - Vulnérabilités dans le code (injection, escalade, contournement)
   - Fragilités architecturales (surface d'attaque, trust boundaries)
   - Qualité des dépendances (audit, licences, provenance)
   - Documentation et commentaires (information leakée, guides trompeurs)
   - Outillage de build (CI, compilation, distribution du binaire)

3. **Relecture agressive** de la grille par le superviseur — c'est un référentiel permanent, il doit être irréprochable.

4. **Premier passage** de la grille sur tout le projet → `docs/audits/security-audit.md` avec :
   - Points à résoudre (classés par sévérité)
   - Recommandations de long terme
   - Conclusion à l'attention de Julien

5. **Corrections** via le cycle Generator/Evaluator.

### Politique de re-validation après corrections

- **Corrections mineures** (coquilles, renommages, commentaires) : relancer les tests, vérification de surface.
- **Corrections significatives** (à l'appréciation du superviseur) : relancer les phases en aval. Si la correction touche l'architecture ou la sécurité, tout reprendre depuis la phase 1.

### Politique permanente pour les futures versions et PR

Cette stratégie définit l'ordre de validation pour toute modification ultérieure :

1. **Tests** — le comportement est-il verrouillé ?
2. **Revue de code et simplification** — la complexité est-elle justifiée ?
3. **Sécurité** — le code est-il sûr ?

Si une étape n'est pas validée, on recommence depuis le début. Les référentiels créés (grilles d'analyse, scénarios) sont des documents vivants maintenus au fil des versions.

## Conséquences

### Positives

- Confiance élevée dans la qualité du code publié
- Référentiels réutilisables pour les futures versions
- Traçabilité complète (audits, scénarios, grilles)
- Le cycle Generator/Evaluator empêche l'auto-complaisance

### Négatives

- Coût élevé en tokens (3 phases, chacune avec generator + evaluator)
- Temps d'exécution significatif (estimation : 6-10h de superviseur)
- Risque de boucle si les corrections d'une phase invalident les précédentes

### Risques mitigés

- Le premier passage établit le plafond haut de consommation de tokens — les suivants seront moins chers car les référentiels existent
- La parallélisation sera possible une fois les référentiels stabilisés — pas pour cette première fois
