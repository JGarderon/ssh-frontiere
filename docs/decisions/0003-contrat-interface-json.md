# ADR 0003 — Contrat d'interface : entrées/sorties JSON et codes de sortie

**Date** : 2026-03-15
**Statut** : Proposée
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : Exercice d'alignement 001, sections 2 (Contrat d'interface) et 8.1 (Format des retours)
**Voir aussi** : ADR 0001 (format configuration TOML — section identités et secrets), ADR 0002 (modèle de secrets — masquage SHA-256 optionnel, protocole challenge-response), ADR 0004 (struct Context — Decision::Execute et Decision::Reject portent les codes de sortie)

---

## Contexte

SSH Frontière est consommé par des acteurs hétérogènes :
- **Runners Forgejo Actions** : scripts shell qui parsent stdout
- **Agents LLM** : besoin de JSON structuré pour le raisonnement
- **Humains** : besoin de messages lisibles sur stderr pour le debug

Le projet forge-generale prévoit un **bouchon** (stub bash) de ssh-frontiere pendant sa propre phase de développement. Ce bouchon doit respecter le même contrat d'interface que le binaire Rust final.

L'exercice d'alignement 001 a établi la **position B — structuré JSON** (décision Julien BO, section 8.1) : enveloppe JSON sur stdout, pas de gabarisation.

---

## Options

### Format de sortie stdout

| Option | Description | Consommateur cible |
|--------|-------------|-------------------|
| A — Opaque | stdout/stderr bruts du processus enfant | Scripts shell |
| B — JSON structuré | Enveloppe JSON à champs fixes (`status_code`, `status_message`, `stdout`, `stderr`) | LLM, automation |
| C — Gabarisation | Templates dans les règles pour formater la sortie | Cas spécifiques |

### Codes de sortie

| Option | Description |
|--------|-------------|
| A — Passthrough | Le code de la commande enfant est retourné tel quel |
| B — Codes réservés | Plage réservée pour les erreurs ssh-frontiere, passthrough pour le reste |
| C — Toujours 0 ou 1 | Simplifié, détails dans le JSON |

### Format d'entrée

| Option | Description |
|--------|-------------|
| A — Positionnelle | `<action> [args...] <domaine>` |
| **B — Domaine d'abord** | **`<domaine> <action> [args...]`** |
| C — Action d'abord | `<action> <domaine> [args...]` |

---

## Décision

### Entrée : `<domaine> <action> [arguments...]` *(retour Julien BO, 2026-03-15)*

La commande SSH suit le format :

```
ssh user@host "<domaine> <action> [arguments...]"
```

**Ordre : domaine d'abord, puis action, puis arguments** *(décision Julien BO)*. C'est l'ordre naturel de la langue : « sur quoi ? » puis « que faire ? ». Exemples : `mastodon backup-full`, pas `backup-full mastodon`.

**Résolution du domaine** :
- Le domaine est **toujours le premier token** (sauf pour les sous-commandes built-in)
- Les sous-commandes built-in (`help`, `list`) n'ont pas de domaine

**Authentification RBAC dans la commande** *(lien ADR 0002, Phase 3+)* :
Quand le protocole challenge-response sera implémenté, le format de commande intégrera la notion de session d'authentification. Deux approches envisagées (cf. ADR 0002) :
- Commande `auth` comme sous-commande built-in pour initier le challenge-response
- Token de session temporaire passé en argument spécial après authentification

Le format exact sera défini dans une ADR dédiée Phase 3. Le MVP utilise uniquement l'authentification SSH (`--level`).

**Exemples** :

```bash
# Domaine + action
ssh runner@host "mastodon backup-full"
ssh runner@host "forgejo backup-config"
ssh runner@host "infra healthcheck"

# Action avec arguments
ssh runner@host "forgejo deploy latest"

# Découverte (sous-commandes built-in, pas de domaine)
ssh runner@host "help"
ssh runner@host "help mastodon"    # help par domaine
ssh runner@host "list"
```

**Justification** *(Julien BO)* : le domaine en premier est l'ordre naturel. On dit « mastodon backup-full » comme on dit « la base, sauvegarde-la ». Le périmètre fonctionnel (domaine) cadre le contexte avant l'action.

### Sortie : format fixe à 4 champs *(retour Julien BO, 2026-03-15)*

Chaque invocation produit **exactement un objet JSON** sur stdout avec **toujours 4 champs** :

| Champ | Type | Description |
|-------|------|-------------|
| `status_code` | int | Code de statut entier (0 = succès, codes réservés sinon) |
| `status_message` | string | Message textuel décrivant le statut |
| `stdout` | string \| null | Sortie standard de la commande. **`null` si pas d'exécution** (pas une chaîne vide) |
| `stderr` | string \| null | Sortie d'erreur de la commande. **`null` si pas d'exécution** (pas une chaîne vide) |

**Distinction `null` vs chaîne vide** *(décision Julien BO)* :
- `null` = la commande n'a **pas été exécutée** (rejet, erreur de config, etc.)
- `""` (chaîne vide) = la commande a été exécutée mais n'a **rien produit** sur ce flux

Cette distinction est sémantiquement importante : un consommateur (LLM, script) peut différencier « pas de sortie car pas d'exécution » de « exécution sans sortie ».

**Limites de caractères** :
- `stdout` et `stderr` sont tronqués à une limite configurable dans `[global]` :
  ```toml
  [global]
  max_stdout_chars = 65536    # défaut : 64 Ko
  max_stderr_chars = 16384    # défaut : 16 Ko
  max_output_chars = 131072   # maximum indépassable (128 Ko), hard limit
  ```
- Si la sortie dépasse la limite, elle est tronquée et `status_message` l'indique

**Exemples** :

```json
// Commande exécutée avec succès
{
  "status_code": 0,
  "status_message": "executed",
  "stdout": "Backup completed successfully\n",
  "stderr": ""
}

// Commande rejetée (pas d'exécution → stdout/stderr null)
{
  "status_code": 128,
  "status_message": "rejected: unknown action 'foo'",
  "stdout": null,
  "stderr": null
}

// Niveau insuffisant (pas d'exécution → null)
{
  "status_code": 131,
  "status_message": "rejected: insufficient level (ops required, read granted)",
  "stdout": null,
  "stderr": null
}

// Commande exécutée mais échouée
{
  "status_code": 1,
  "status_message": "executed",
  "stdout": "",
  "stderr": "backup-config.sh: permission denied\n"
}

// Timeout (exécution partielle → sortie capturée si disponible)
{
  "status_code": 130,
  "status_message": "timeout after 300s: mastodon backup-full",
  "stdout": "Starting backup...\n",
  "stderr": null
}
```

**Champs additionnels** : des champs informatifs supplémentaires peuvent être ajoutés à l'objet JSON (comme `duration_ms`, `domain`, `action`) mais les 4 champs ci-dessus sont **toujours présents** et constituent le contrat minimal.

**Cas de découverte** (`help`, `list`) : le JSON retourné utilise le même format à 4 champs. Le contenu de la découverte est dans `stdout` (sérialisé en JSON imbriqué) :

```json
{
  "status_code": 0,
  "status_message": "ok",
  "stdout": "{\"domains\":{\"forgejo\":{\"description\":\"Forge Git infrastructure\",\"actions\":{\"backup-config\":{\"description\":\"Sauvegarde la configuration\",\"level\":\"ops\",\"args\":[]}}}}}",
  "stderr": null
}
```

### Stderr : messages humains courts

En complément du JSON sur stdout, stderr reçoit un message court en cas d'erreur :

```
ssh-frontiere: rejected: unknown action 'foo'
ssh-frontiere: rejected: insufficient level (ops required, read granted)
ssh-frontiere: timeout after 300s: mastodon backup-full
ssh-frontiere: error: config file not found: /etc/ssh-frontiere/config.toml
```

Format : `ssh-frontiere: <type>: <message>`. Une seule ligne. Pas de stack trace, pas de chemin interne (sauf config).

### Codes de sortie : plage réservée (option B)

| Code | Signification |
|------|---------------|
| 0 | Succès (commande exécutée, exit 0) |
| 1-127 | Code de sortie de la commande enfant (passthrough) |
| 128 | Commande refusée (inconnue, argument invalide, entrée non grammaticale) |
| 129 | Erreur de configuration (fichier absent, TOML invalide, validation échouée) |
| 130 | Timeout (commande tuée après dépassement) |
| 131 | Niveau insuffisant (RBAC) |

**Note** : les codes 128+ sont conventionnellement utilisés par les shells pour les signaux (128+N = signal N). SSH Frontière n'est pas un shell — ces codes sont réutilisés sans ambiguïté car ssh-frontiere est le seul processus à les émettre.

### Contrat pour le bouchon forge-generale

Le bouchon bash doit respecter :
1. Accepter une commande via `SSH_ORIGINAL_COMMAND` ou `$1`
2. Retourner un JSON valide sur stdout avec les 4 champs obligatoires : `{"status_code": 0, "status_message": "executed", "stdout": "...", "stderr": null}`
3. Retourner le code de sortie 0
4. Logger l'invocation (fichier ou syslog)

Le bouchon n'implémente **pas** : RBAC, validation d'arguments, timeouts, domaines, challenge-response.

---

## Conséquences

### Positives

- Format fixe à 4 champs : contrat minimal clair et testable
- Distinction `null` / chaîne vide : sémantique précise pour les consommateurs
- Limites de caractères configurables avec maximum indépassable : protection contre les sorties massives
- Ordre domaine→action naturel et lisible
- JSON natif pour les agents LLM (pas besoin de parser du texte)
- Stderr préservé pour le debug humain (en plus du `stderr` JSON)
- Codes de sortie structurés, distinguant erreur d'exécution et erreur de politique
- Le bouchon est simple à implémenter et garanti compatible

### Négatives

- Le JSON enveloppant stdout ajoute un overhead (sérialisation de la sortie complète)
- La troncature à `max_output_chars` peut faire perdre de l'information — le `status_message` doit l'indiquer clairement
- Le code 128 en shell signifie conventionnellement « signal 0 reçu » — la réutilisation peut surprendre des opérateurs habitués aux conventions shell

### Risques

- Si stdout de la commande enfant contient du JSON invalide ou des caractères de contrôle, `serde_json` les échappera correctement (risque faible)
- La limite `max_output_chars` (128 Ko hard limit) peut être insuffisante pour certains dumps — à surveiller, mais protège la mémoire
- Le bouchon et le binaire Rust doivent rester synchronisés sur le contrat — les tests d'intégration sont le filet de sécurité

---

## Attribution

- **Julien (BO)** : choix position B (JSON structuré), ordre domaine→action→arguments, format retour 4 champs fixes (status_code/status_message/stdout/stderr), distinction null vs vide, limites de caractères, concept du bouchon forge-generale, lien RBAC challenge-response
- **Claude (PM/Tech Lead)** : structure des champs JSON, codes de sortie, format stderr, contrat bouchon, formalisation des limites configurables
- **Agents Claude Code** : implémentation, tests de conformité
