# ADR 0006 — Protocole d'entêtes unifié

**Date** : 2026-03-15
**Statut** : Acceptée
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : Exercice d'alignement 002, ADR 0002 (secrets et challenge-response), ADR 0003 (contrat d'interface JSON), ADR 0004 (contexte et résolution), ADR 0005 (SHA-256 maison)
**Phase** : 3 — Multi-consommateurs et agents

---

## Contexte

SSH Frontière fonctionne en mode **one-shot** depuis les Phases 1-2 : une connexion SSH = une commande via `SSH_ORIGINAL_COMMAND` = une réponse JSON = fermeture. Ce modèle est insuffisant pour la Phase 3 :

1. **Authentification RBAC par token** (ADR 0002, protocole challenge-response) : nécessite au minimum 2 échanges (challenge + preuve). En one-shot, cela impliquerait 2 connexions SSH successives avec stockage temporaire côté serveur.

2. **Mode session** : certains consommateurs (agents LLM, opérateurs humains) bénéficient d'un mode multi-commandes par connexion.

L'exercice d'alignement 002 (validé par Julien BO, 2026-03-15) a unifié ces deux besoins en un **protocole d'entêtes** inspiré de HTTP, remplaçant les 2 ADR séparées initialement prévues (TODO-011 et TODO-012).

### Changement de paradigme d'invocation

Jusqu'ici, ssh-frontiere recevait la commande via `-c` (login shell mode) ou `SSH_ORIGINAL_COMMAND`. Avec le protocole d'entêtes, **stdin/stdout deviennent le canal de protocole**. La commande n'est plus passée en argument mais envoyée par le client via stdin avec le préfixe `$`.

Le paramètre `-c` et `SSH_ORIGINAL_COMMAND` sont **ignorés** pour l'exécution de commandes. Les arguments `--level` et `--config` restent fonctionnels (passés via `command=` dans `authorized_keys`).

---

## Décision

### 1. Protocole ligne par ligne — 4 préfixes

Chaque ligne échangée entre client et serveur commence par un préfixe qui définit sa sémantique :

| Préfixe | Rôle | Direction | Description |
|---------|------|-----------|-------------|
| `+` | **Configure** | Bidirectionnel | Paramètres de session, capabilities, authentification, challenge |
| `#` | **Commente** | Bidirectionnel | Information libre, contexte, debug, aide |
| `$` | **Ordonne** | Client → serveur | Commande à exécuter (`domaine action [args]`) |
| `>` | **Répond** | Serveur → client | Réponse JSON (format 4 champs ADR 0003) |

**Format** : préfixe suivi d'un espace, puis contenu. Exemple : `$ forgejo healthcheck`.

**Pas de rétrocompatibilité** : toute ligne sans préfixe reconnu est une erreur de protocole. Il n'y a aucun client déployé, donc aucune dette (décision Julien BO).

> Principe fondateur (Julien BO) : « C'est gracieux, lisible par l'homme, le LLM, programmatiquement. »

### 2. Flux de connexion

```
[connexion SSH ouverte]

=== Phase 1 : bannière serveur (immédiate, obligatoire) ===
# ssh-frontiere 0.1.0
+capabilities rbac, session, help
+challenge nonce=a7f3b2c9e1d4f6a8b3c7d2e5f1a9b4c6
# type "$ help" for available commands

=== Phase 2 : entêtes client (0 à N lignes + ligne vide) ===
+auth token=runner proof=e4d5a1b3c7...
# healthcheck sur forgejo
                                          ← ligne vide = fin des entêtes

=== Phase 3 : commande ===
$ forgejo healthcheck

=== Phase 4 : réponse ===
> {"status_code": 0, "status_message": "executed", "stdout": "OK\n", "stderr": null}

=== Fin (ou boucle si +session keepalive) ===
```

#### Bannière serveur

**Obligatoire**, envoyée immédiatement à l'ouverture de connexion (comme SMTP/FTP). Contient :

- `#` version du programme
- `+capabilities` : liste des fonctionnalités supportées (contenu **conditionnel** selon la configuration : `rbac` n'apparaît que si `[auth.tokens]` est configuré, `session` est toujours annoncé)
- `+challenge nonce=<hex>` : nonce pour le challenge-response — **présent uniquement si `[auth.tokens]` est configuré**. Si aucun token n'est défini, la ligne `+challenge` est omise et `rbac` n'apparaît pas dans les capabilities
- `#` messages d'aide optionnels

Le serveur **flush stdout** après la bannière complète pour garantir que le client reçoit toutes les lignes avant d'envoyer ses entêtes.

> Décision Julien (BO) : « Obligatoire, sans aucun doute possible. Le protocole a une architecture d'échanges définie, comme HTTP. Le serveur s'annonce toujours. »

#### Fin de la phase d'entêtes

La phase d'entêtes client se termine par une **ligne vide** : une ligne contenant uniquement `\n` (pas d'espaces, pas de tabulation). Cela produit `\n\n` pour un programme, un double « Entrée » pour un humain.

La ligne vide est **optionnelle** avant la première commande. Le parseur accepte la commande directement après les entêtes — les 4 préfixes explicites (+, #, $, >) rendent la ligne vide redondante pour la détection de la transition entêtes→commande.

> Décision Julien (BO) : « Ligne vide. Tout à fait acceptable et naturel. »
>
> Addendum (2026-03-18, TODO-027) : la ligne vide entre entêtes et commande est optionnelle. Le parseur bascule en mode commande dès qu'il rencontre une ligne `Text` (sans préfixe reconnu ou avec `$`), qu'une ligne vide ait été envoyée ou non. Ce comportement est par design depuis la Phase 3 — `read_headers()` ignore les `EmptyLine` avec `continue`. La ligne vide reste acceptée pour la lisibilité.

#### Commandes et réponses

- `$ domaine action [args]` : reprend le format d'entrée ADR 0003 avec le préfixe `$`
- `> {json}` : encapsule la réponse JSON à 4 champs (ADR 0003) avec le préfixe `>`
- Les commandes built-in `$ help`, `$ list`, `$ exit` sont reconnues
- `help` sans préfixe `$` est une commande spéciale de découverte : elle retourne du texte humain (pas du JSON) via les commentaires serveur `#>`. C'est le point d'entrée pour un humain qui ne connaît pas le protocole (TODO-028, addendum 2026-03-18).
- `$ exit` en mode non-session : le serveur répond `> {"status_code": 0, "status_message": "ok", ...}` et ferme normalement (la connexion se fermait de toute façon)

#### Canal stderr

Le canal stderr (distinct de stdout) continue d'être utilisé pour les messages humains courts au format `ssh-frontiere: type: message` (ADR 0003). stderr n'est **pas** partie du protocole d'entêtes — il est indépendant et préservé pour le debug.

### 3. Challenge-response intégré

Le protocole challenge-response (ADR 0002, précisé dans l'alignement 002) est intégré dans les entêtes. La séquence cryptographique est :

```
empreinte( chiffré( secret + T ) )
```

L'empreinte **enveloppe** le chiffrement — l'empreinte est à la fin, pas au milieu.

> Formulation exacte de Julien (BO) : « Pour le protocole de challenge, ce que je veux c'est : empreinte( chiffré( (secret + T) ) ) → l'empreinte est à la fin. »

#### Flux détaillé

```
1. Serveur → Client : +challenge nonce=<T_hex>
   T = 16 octets aléatoires (/dev/urandom), encodé hex (32 caractères)

2. Client : concatène secret + T (octets bruts)
   plaintext = secret_bytes || T_bytes

3. Client : chiffre le plaintext avec le secret comme clé
   keystream = SHA-256(secret || 0x00) || SHA-256(secret || 0x01) || ...
   ciphertext = plaintext XOR keystream[:len(plaintext)]

4. Client : calcule l'empreinte du résultat chiffré
   proof = SHA-256(ciphertext)

5. Client → Serveur : +auth token=<id> proof=<proof_hex>
```

Le serveur, connaissant le secret associé au token `<id>` (stocké dans `[auth.tokens]`), effectue le même calcul et compare les preuves en **temps constant** (protection timing side-channel).

#### Primitives cryptographiques

| Primitive | Algorithme | Justification |
|-----------|-----------|---------------|
| Empreinte | SHA-256 (FIPS 180-4, implémentation maison, cf. ADR 0005) | Standard, déjà dans le codebase, testée NIST |
| Chiffrement | XOR stream cipher, keystream = SHA-256(secret ‖ counter) | Zéro dépendance, déterministe, proportionné à l'usage |
| Nonce | 16 octets aléatoires `/dev/urandom` | 128 bits d'entropie, collision à ~2^64 connexions |

**Keystream CTR** : si le plaintext dépasse 32 octets, le keystream est étendu par blocs de 32 octets via `SHA-256(secret || counter)` avec counter = 0x00, 0x01, etc. En pratique, un secret ≤ 64 octets + nonce 16 octets = 80 octets max → 3 blocs de keystream suffisent.

#### Note sur ADR 0005

La condition de révision de l'ADR 0005 est déclenchée : SHA-256 est désormais utilisé pour l'authentification. L'implémentation maison reste acceptable :

- Le proof est un hash de données chiffrées — un bug de hash produirait un proof incorrect (déni de service par échec d'auth), pas une fuite de secret
- Forger un proof nécessiterait une attaque en pré-image sur SHA-256 (infaisable indépendamment de l'implémentation)
- L'implémentation passe les vecteurs NIST complets incluant les cas limites (TODO-008)
- La politique zéro dépendance s'applique toujours

#### Propriétés de sécurité

- **Secret jamais transmis** : seul le proof (hash) transite — irréversible
- **Replay impossible** : le nonce T est unique par connexion
- **Double protection** : l'attaquant voit T (public) et le proof. Pour forger le proof, il faudrait inverser le hash (impossible) ET le chiffrement (nécessite le secret)
- **Timing résistant** : comparaison en temps constant du proof côté serveur

### 4. Authentification : couche supplémentaire, pas prérequis

**L'authentification RBAC n'est pas obligatoire.** SSH est la couche de confiance de base — une fois connecté, le client est identifié par sa clé SSH et son `--level` configuré dans `authorized_keys`.

Le `+auth` est une **couche supplémentaire** pour les actions qui exigent un niveau RBAC supérieur :

- **Sans `+auth`** : le client opère au niveau de base accordé par `--level`
- **Avec `+auth` valide** : le niveau effectif est le **maximum** entre `--level` et le niveau du token
- **Pas de concept d'« anonyme »** : on est toujours identifié par SSH

> Julien (BO) : « L'authentification n'est pas une obligation mais une configuration possible. L'authentification est posée par le RBAC, mais le RBAC n'impose pas d'avoir un mode "anonyme". »

#### Validation du `+auth`

La validation est **immédiate** (eager) : dès que le serveur lit une ligne `+auth`, il vérifie le proof et met à jour le contexte d'authentification ou incrémente le compteur d'échecs. Un commentaire `#` de feedback est envoyé au client.

### 5. Le `+auth` comme préambule

Le `+auth` n'est pas un état global de session — c'est un **préambule aux commandes qui suivent**. Il vaut jusqu'au prochain `+auth` ou jusqu'à la fin de session.

On peut avoir **plusieurs `+auth` différents** dans une même session :

```
+auth token=runner proof=abc123

$ forgejo backup-config
> {...}

+auth token=admin proof=def456

$ forgejo deploy latest
> {...}
```

Si une action exige un niveau RBAC supérieur au niveau effectif courant, elle est rejetée avec le code 131 (`EXIT_INSUFFICIENT_LEVEL`).

### 6. Mode session (opt-in)

Le mode session est un **entête de configuration**, pas un changement de paradigme :

```
+session keepalive
```

Si présent dans les entêtes client, le serveur **ne ferme pas** après la première réponse. Le client peut envoyer d'autres lignes `+`, `#` ou `$`.

**Fin de session** (au choix du client) :
- `$ exit` : commande built-in de fermeture explicite
- EOF : fermeture de stdin

**Timeout de session** : configurable dans `[global]` via `timeout_session`. Après inactivité prolongée (aucune ligne reçue du client), le serveur ferme la connexion avec un commentaire `# session timeout`.

**Flux en mode session** : après chaque réponse `>`, le serveur attend la prochaine ligne. Pas de ligne vide requise entre les commandes successives — la ligne vide n'est obligatoire qu'à la fin de la phase d'entêtes initiale.

> Décision Julien (BO) : « `$ exit` ou EOF, au choix du client. Deux mécanismes équivalents. »

### 7. Protection contre les tentatives échouées

Au bout de `max_auth_failures` échecs d'authentification (défaut : 3), le serveur :

1. **Coupe la session** immédiatement
2. **Journalise** l'événement (log JSON avec IP source, token tenté, horodatage)
3. **Exécute optionnellement** une commande configurable (`ban_command`)

```
→ +auth token=runner proof=MAUVAIS
← # auth failed (1/3)
→
→ $ infra maintenance
← > {"status_code": 131, "status_message": "rejected: insufficient level ...", ...}

→ +auth token=runner proof=ENCORE_MAUVAIS
← # auth failed (2/3)
→
→ $ infra maintenance
← > {"status_code": 131, "status_message": "rejected: insufficient level ...", ...}

→ +auth token=runner proof=TOUJOURS_MAUVAIS
← # auth failed (3/3) — session terminated
[connexion fermée par le serveur]
```

Le compteur d'échecs est **par connexion** (pas persistant entre connexions). La commande de ban reçoit l'IP source via le placeholder `{ip}` et est exécutée via `std::process::Command` (pas de shell — protection injection).

### 8. Capabilities

La bannière serveur annonce ses capabilities :

```
+capabilities rbac, session, help
```

- **Purement informatif** : le serveur annonce, le client s'adapte
- **Pas de négociation** : le client ne peut pas demander une capability non annoncée
- Format : mots séparés par `, ` (virgule + espace) pour la lisibilité humaine
- MVP Phase 3 : `rbac`, `session`, `help`

> Julien (BO) : « Met des espaces, c'est plus lisible pour l'humain. »

### 9. Commentaires (#)

Les lignes `#` sont **bidirectionnelles et libres**. Canal de debug/contexte sans interférence avec le protocole.

- **Serveur** : version, aide, feedback d'auth, contexte d'exécution
- **Client** : intentions, documentation pour l'audit
- **Logging** : **configurable** via `log_comments` dans `[global]`. L'administrateur choisit de tracer les commentaires (enrichit l'audit) ou de les garder éphémères
- **Impact protocole** : aucun. Les `#` n'affectent ni l'authentification, ni l'exécution

### 10. Gestion des erreurs de protocole

| Cas | Comportement |
|-----|-------------|
| Ligne sans préfixe reconnu | Erreur de protocole, log, session fermée |
| `$` avec commande invalide | Réponse `>` avec rejet (codes existants ADR 0003) |
| `+` avec directive inconnue | Ignorée silencieusement (compatibilité future) |
| `+auth` avec format invalide | Comptabilisée comme échec d'auth |
| EOF inattendu (phase d'entêtes) | Session fermée proprement |
| Ligne trop longue (> 4096 chars) | Erreur de protocole, session fermée |

**Nouveau code de sortie** : `132` (`EXIT_PROTOCOL_ERROR`) pour les erreurs de protocole.

### 11. Impact sur la configuration TOML

#### Nouvelles entrées dans `[global]`

```toml
[global]
# ... champs existants (log_file, default_timeout, etc.) ...
timeout_session = 3600    # timeout session en secondes (défaut: 3600)
max_auth_failures = 3     # tentatives d'auth max avant coupure (défaut: 3)
log_comments = false      # journaliser les lignes # (défaut: false)
ban_command = ""          # commande de ban optionnelle (défaut: vide = désactivé)
                          # Exécutée via std::process::Command (pas de shell)
                          # Placeholder {ip} remplacé par l'IP source (SSH_CLIENT)
                          # Exemple: "/usr/local/bin/ban-ip.sh {ip}"
```

#### Nouvelle section `[auth]`

```toml
[auth]
# Section optionnelle — si absente, pas d'auth RBAC disponible

[auth.tokens.runner-forge]
secret = "b64:c2VjcmV0LXJ1bm5lci1mb3JnZQ=="   # secret encodé base64 (ADR 0002, niveau 3)
level = "ops"

[auth.tokens.agent-claude]
secret = "b64:c2VjcmV0LWFnZW50LWNsYXVkZQ=="
level = "read"

[auth.tokens.admin-julien]
secret = "b64:c2VjcmV0LWFkbWluLWp1bGllbg=="
level = "admin"
```

**Format du secret** : encodé en base64 avec préfixe `b64:` (ADR 0002, niveau 3). Le serveur décode le secret au chargement pour le challenge-response.

**Changement par rapport au format réservé dans ADR 0001/0002** : le champ est `secret` (pas `hash`). Le serveur a besoin du secret en clair (décodé) pour effectuer le même calcul que le client. L'encodage base64 est une protection sociale (ADR 0002, niveau 3).

**Niveau par token** : chaque token définit un niveau RBAC (`read`, `ops`, `admin`). Le niveau effectif lors d'un `+auth` valide est le maximum entre `--level` et le niveau du token.

#### Validation au chargement

En plus des validations existantes (ADR 0001), le chargement vérifie :

- Si `[auth]` est présent : chaque token a un `secret` et un `level` valide
- Les secrets `b64:` sont décodables en base64 valide
- Les noms de tokens sont des identifiants valides (alphanumérique + tiret)

---

## Conséquences

### Positives

- **Un seul protocole** pour l'auth RBAC et le mode session — pas deux mécanismes séparés
- **Challenge-response en 1 connexion** : résout le problème identifié dans ADR 0002 (pas besoin de 2 connexions ni de stockage temporaire du nonce côté serveur)
- **Lisibilité triple** : humain (préfixes lisibles), LLM (structure claire), machine (parsing ligne par ligne)
- **Extensible** : les directives `+` inconnues sont ignorées (compatibilité future)
- **Zéro nouvelle dépendance** : SHA-256 maison + XOR stream cipher

### Négatives

- **Rupture avec le mode one-shot actuel** : stdin/stdout deviennent le canal de protocole, `-c` et `SSH_ORIGINAL_COMMAND` ne sont plus utilisés pour l'exécution de commandes
- **Complexité accrue** du programme : boucle de lecture, machine à états, gestion de session
- Le cipher XOR-SHA256 n'est pas un standard reconnu (acceptable pour l'usage challenge-response, pas recommandé pour d'autres usages)
- Les tests d'intégration existants doivent être réécrits pour utiliser le protocole

### Risques

- Le passage au mode protocole change fondamentalement l'interaction côté client. Les scripts utilisant `ssh user@host "commande"` doivent migrer vers le protocole (stdin/stdout)
- La gestion de session introduit de l'état (compteur d'auth, contexte d'auth courant, mode session) dans un programme jusqu'ici stateless
- Le cipher maison, bien que simple, nécessite des tests exhaustifs pour éviter les cas limites

---

## Attribution

- **Julien (BO)** : concept du protocole d'entêtes unifié (inspiré HTTP), définition des 4 préfixes (+, #, $, >), bannière obligatoire, ligne vide comme séparateur, format `+auth token=<id> proof=<hex>`, précision `empreinte( chiffré( secret + T ) )` — l'empreinte à la fin, l'auth n'est pas une obligation, `+auth` comme préambule (pas d'état global), mode session opt-in (`+session keepalive`), fin de session `$ exit` ou EOF, protection 3 tentatives + ban, capabilities avec espaces, pas de négociation, commentaires # bidirectionnels loggés selon config, pas de rétrocompatibilité
- **Claude (PM/Tech Lead)** : intégration du challenge-response dans les entêtes (résout le problème ADR 0002), primitives cryptographiques (XOR-SHA256 stream cipher), code de sortie protocole (132), gestion des lignes invalides, format TOML détaillé `[auth.tokens]`, propriétés de sécurité, validation eager du `+auth`, sémantique du niveau effectif (max)
- **Agents Claude Code** : implémentation, tests
