# ADR 0012 — Texte libre et ressources

**Date** : 2026-03-19
**Statut** : Accepted (2026-03-20)
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : ADR 0006 (protocole d'en-têtes), ADR 0009 (arguments nommés et valeurs par défaut), ADR 0003 (contrat d'interface JSON), ADR 0011 (streaming)
**Voir aussi** : Exercice d'alignement 004, TODO-029

---

## Contexte et problème

SSH Frontière permet aujourd'hui d'envoyer des **commandes courtes** avec des **arguments nommés** (`key=value`, valeurs courtes et prédéfinies dans le config.toml). Ce modèle est insuffisant pour deux besoins identifiés :

1. **Arguments texte libre** : certaines actions nécessitent une valeur arbitrairement longue, non prédéfinie dans la configuration (ex: un message, une note, un extrait de log).

2. **Envoi de ressources textuelles** : certaines actions doivent recevoir un contenu multiligne accompagnant la commande (ex: un fichier de configuration TOML, un script bash, un payload JSON/YAML).

### Cas d'usage concrets

| Cas | Description | Taille typique |
|-----|-------------|----------------|
| Déployer une configuration | Envoyer un `config.toml` ou `.env` mis à jour | 1-50 Ko |
| Envoyer un script | Transmettre un script bash à exécuter | 1-10 Ko |
| Payload JSON/YAML | Document structuré à traiter par une action | 1-100 Ko |
| Contenu d'un template | Remplir un fichier à partir d'un modèle | 1-50 Ko |
| Message ou note | Texte libre multilingue (notification, log) | 100 octets - 5 Ko |

### Contraintes

- Le protocole actuel est orienté lignes, avec `.` seul sur une ligne comme terminateur de commande
- Le contenu multiligne nécessite un mécanisme de délimitation qui ne conflicte pas avec le terminateur existant
- SSH Frontière est orienté texte — l'envoi de contenu purement binaire (hors caractères imprimables) n'est pas le cas d'usage principal. Cependant l'usage des arguments `size` et/ou `stop` n'interdit pas un envoi de caractères non imprimables et qui ne respectent pas le protocole.
- Le programme est synchrone et one-shot (pas de streaming bidirectionnel)
- La compatibilité avec le mode session (ADR 0006, `+session keepalive`) doit être préservée

---

## Décisions

### D1 — Délimitation : section body avec trois modes (`size=`, `stop=`, défaut)

Le header `+body` annonce qu'un bloc de contenu suit la commande. Il supporte **trois modes de délimitation**, contrôlables par des paramètres optionnels du header :

#### Mode 1 — Taille fixe : `+body size=N`

Le nombre d'octets à lire est déclaré à l'avance. SSH Frontière lit exactement `N` octets puis cesse. Si `N` dépasse `max_body_size` de l'action, le body est **rejeté intégralement** (pas de lecture partielle).

```
+body size=1024
mastodon deploy config env=production
.
<exactement 1024 octets de contenu, sans terminateur>
```

#### Mode 2 — Séparateur explicite : `+body stop="FIN"`

Le contenu est lu jusqu'à ce que la chaîne `stop` soit trouvée seule sur une ligne. Le séparateur ne fait pas partie du body.

```
+body stop="---END---"
mastodon deploy config env=production
.
contenu du fichier config
sur plusieurs lignes
---END---
```

#### Mode 3 — Défaut : `+body` (sans paramètre)

Le comportement par défaut réutilise le terminateur existant `\n.\n` (un `.` seul sur une ligne). C'est le mode le plus simple, compatible avec le protocole actuel.

```
+body
mastodon deploy config env=production
.
contenu du fichier config
sur plusieurs lignes
.
```

#### Combinaison `size=` + `stop=`

Si les deux sont précisés (`+body size=4096 stop="---END---"`), le premier des deux atteint termine la lecture. Cas d'usage : quand la taille est estimée mais pas exacte (streaming amont avec taille indicative).

#### Priorité et résolution

| Paramètres | Comportement |
|------------|-------------|
| `+body` | Lit jusqu'à `\n.\n` |
| `+body size=N` | Lit exactement N octets |
| `+body stop="X"` | Lit jusqu'à la ligne `X` |
| `+body size=N stop="X"` | Le premier des deux atteint termine la lecture |

Le header `+body` est optionnel — les commandes sans body fonctionnent exactement comme avant.

*(Décision Julien / BO — 2026-03-20)*

### D2 — Limites de taille : par action (`max_body_size`)

Chaque action peut déclarer une limite de taille pour le body reçu, via un champ `max_body_size` dans sa configuration TOML :

```toml
[domains.mastodon.actions.deploy]
description = "Déploiement avec configuration"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain}"
max_body_size = 65536   # 64 Ko

[domains.mastodon.actions.deploy.args]
env = { type = "enum", values = ["production", "staging"], default = "production" }
```

**Valeur par défaut** : 65536 octets (64 Ko) si non spécifié.

**Comportement en cas de dépassement** : rejet immédiat avec erreur explicite (pas de troncage silencieux). Le message d'erreur indique la taille reçue et la taille maximale autorisée.

**Pas de limite globale supplémentaire** : la granularité par action est suffisante. Une action qui accepte des fichiers de configuration peut avoir une limite de 50 Ko, tandis qu'une action de backup peut accepter 10 Mo. La limite par défaut protège les actions qui n'ont pas défini explicitement de seuil.

### D3 — Typage : pas de typage, body = stdin exclusivement

SSH Frontière reçoit le contenu du body comme une chaîne de caractères brute et le passe au **stdin** du processus enfant — et **seulement** au stdin. Pas d'écriture sur disque, pas de fichier temporaire, pas de variable d'environnement. Si le stdin de la commande cible est fermé ou indisponible, c'est une **erreur majeure** signalée explicitement (code d'erreur dédié).

Pas de typage, pas de header `+ content-type`. L'action qui traite le body sait ce qu'elle reçoit. SSH Frontière est un dispatcher, pas un validateur de contenu.

**Justification** : le principe « l'action sait ce qu'elle reçoit » est cohérent avec le design actuel. Le passage par stdin est le mécanisme Unix le plus naturel et le plus portable — tout script sait lire stdin.

*(Décision Julien / BO — 2026-03-20 : « Si le stdin de la commande cible est fermé, c'est une erreur majeure. »)*

### D4 — Interaction avec les arguments : body séparé des arguments

La commande a ses arguments nommés (ADR 0009) ET un body optionnel. Les deux sont indépendants et complémentaires.

```
+body
mastodon deploy config env=production
.
contenu du fichier config
.
```

L'action reçoit :
- **Arguments résolus** : `{env: "production"}` (mécanisme ADR 0009 inchangé)
- **Body** : le contenu textuel brut (nouvelle donnée)

**Une seule ressource par commande** : pas de mécanisme pour envoyer plusieurs body dans une même commande. Si plusieurs fichiers sont nécessaires, l'action les attend dans un format conteneur (tar, archive) ou le client fait plusieurs appels. La simplicité prime.

**Transmission du body à l'action** : le body est passé au processus enfant via **stdin** (`std::process::Command::stdin(Stdio::piped())`). Le script exécuté peut lire le contenu sur son entrée standard. C'est le mécanisme le plus naturel et le plus portable — tout script sait lire stdin.

### D5 — Arguments texte libre dans la configuration

Un argument peut être déclaré comme « texte libre » dans le config.toml via `free = true`. Cela signifie que la valeur n'est pas prédéfinie — le client envoie ce qu'il veut, sans contrainte de valeurs autorisées.

```toml
[domains.mastodon.actions.notify.args]
message = { free = true }
channel = { type = "enum", values = ["general", "alerts"], default = "general" }
```

| Propriété | Signification |
|-----------|--------------|
| `free = true` | L'argument accepte une valeur arbitraire (texte libre) |
| `free = false` (défaut) | L'argument est contraint par `values` ou `default` (comportement actuel ADR 0009) |

`free = true` est un booléen explicite — il « parle » à l'administrateur : « ici, la valeur est variable et non prédéterminée ». Un argument `free = true` n'a pas besoin de `values` ni de `default` (mais peut avoir un `default`).

**Pas de `nil` explicite** — un booléen est plus clair qu'une valeur nulle pour exprimer l'intention.

*(Décision Julien / BO — 2026-03-20 : « Un true/false qui parle à l'administrateur pour dire c'est une valeur variable ici. »)*

### D6 — Rétrocompatibilité : non requise

SSH Frontière n'a actuellement qu'un seul client (les workflows Forgejo ops via le runner). La rétrocompatibilité n'est pas une contrainte — le protocole peut évoluer librement. Les clients existants seront adaptés en même temps que l'implémentation.

*(Décision Julien / BO — 2026-03-20 : « Pas de rétrocompatibilité nécessaire. On peut tout casser, nous sommes les uniques clients. »)*

### D7 — Session + body : one-shot d'abord, session plus tard

En mode one-shot (par défaut), le body fonctionne sans difficulté :

```
+body
mastodon deploy config
.
contenu
.
```

En mode session (`+session keepalive`), le body est supporté par commande. Le header `+body` précède chaque commande qui l'utilise :

```
+session keepalive
+body
mastodon deploy config
.
contenu du fichier
.
>>> {"command":"mastodon deploy config","status_code":0,...}
mastodon healthcheck
.
>>> {"command":"mastodon healthcheck","status_code":0,...}
+body
mastodon deploy script
.
#!/bin/bash
echo "hello"
.
>>> {"command":"mastodon deploy script","status_code":0,...}
```

Le parseur distingue l'état grâce au header `+body` : après une réponse `>>>`, soit on reçoit `+body` (nouvelle commande avec body), soit on reçoit une commande directement, soit `.` seul (fin de session).

**Pragmatisme** : l'implémentation initiale pourra se limiter au mode one-shot si le besoin session + body s'avère rare. Le support session sera ajouté dans un second temps si nécessaire.

---

## Conséquences

### Positives

- **Extension naturelle du protocole** : le header `+body` s'intègre dans le système de headers existant (ADR 0006) sans nouveau concept fondamental
- **Simplicité** : trois modes de délimitation couvrent tous les cas (taille fixe, séparateur, défaut). Pas d'encodage base64 nécessaire.
- **Composabilité bash** : `printf '+body\nmastodon deploy config\n.\ncontenu\n.\n' | ssh ...` fonctionne naturellement
- **Pas de nouvelle dépendance** : la lecture du body est du parsing de lignes standard (stdlib)
- **Granularité des limites** : chaque action contrôle la taille maximale du body qu'elle accepte
- **Cohérence avec stdin** : passer le body via stdin du processus enfant est le mécanisme Unix le plus naturel

### Négatives

- **Limitation du mode défaut (`.` terminateur)** : le contenu ne peut pas contenir un `.` seul sur une ligne. Contournable en utilisant `size=` ou `stop=` à la place.
- **Complexité du parseur** : en mode session, le parseur doit gérer l'état « j'attends un body » vs « j'attends une commande » — un automate à états supplémentaire
- **Une seule ressource** : pas de support natif pour plusieurs fichiers par commande — le client doit combiner ou faire plusieurs appels

### Risques

- **Collision terminateur** : en mode défaut (`\n.\n`), un contenu contenant `.` seul sur une ligne serait interprété comme fin de body. Atténuation : utiliser `+body size=N` ou `+body stop="..."` pour éviter le problème.
- **Body sans commande** : un client qui envoie `+body` sans commande après. Atténuation : timeout de lecture + erreur de protocole (code 132)
- **Volume mémoire** : le body est lu intégralement en mémoire avant d'être passé au processus enfant. Atténuation : `max_body_size` par action (défaut 64 Ko) limite l'allocation

---

## Alternatives considérées

### Délimitation

| Alternative | Description | Raison du rejet |
|-------------|-------------|-----------------|
| **Content-Length seul** (position A) | Header `+ content-length N` comme unique mode | Intégré dans D1 comme `+body size=N` — pas un rejet mais une option parmi trois |
| **Heredoc** (position B) | Marqueur `+ body <<BOUNDARY` avec terminateur `BOUNDARY` | Intégré dans D1 comme `+body stop="..."` — syntaxe simplifiée sans heredoc bash |
| **Base64** (position C) | Encodage base64 dans un argument nommé `content=base64:...` | +33% de taille, illisible pour le debug, limité par `max_command_length` (~3 Ko réel) |

### Limites de taille

| Alternative | Raison du rejet |
|-------------|-----------------|
| Pas de limite (L1) | Aucune protection contre les payloads excessifs |
| Limite globale seule (L2) | Pas assez granulaire — une action de configuration (50 Ko) et une action de backup (10 Mo) ont des besoins différents |
| Les deux (L4) | Complexité supplémentaire sans bénéfice réel par rapport à la limite par action seule |

### Typage

| Alternative | Raison du rejet |
|-------------|-----------------|
| Typage via header `+ content-type` (T2) | Complexifie le protocole et duplique la responsabilité de validation — le script exécuté est mieux placé pour valider le format de son entrée |

### Interaction avec les arguments

| Alternative | Raison du rejet |
|-------------|-----------------|
| Argument spécial `content=@body` (I2) | Moins propre conceptuellement, mélange le mécanisme d'arguments et le body, plus proche de curl que du modèle SSH Frontière |

---

## Note d'implémentation

L'implémentation de cette ADR fera l'objet d'une **phase dédiée**. Cette ADR documente les décisions de design issues de l'exercice d'alignement 004. La phase d'implémentation inclura :

- Modification du parseur de protocole (`protocol.rs`) pour le header `+body`
- Ajout du champ `max_body_size` dans la configuration (`config.rs`)
- Ajout du champ `free` pour les arguments texte libre (`config.rs`)
- Modification de l'exécution (`dispatch.rs`) pour passer le body via stdin du processus enfant
- Gestion de l'erreur stdin fermé (code d'erreur dédié)
- Tests unitaires, d'intégration et E2E couvrant les scénarios avec body
- Support session + body si le besoin est confirmé

---

## Attribution

- **Julien (BO)** : besoin d'envoi de ressources textuelles, cas d'usage configuration et scripts, trois modes de délimitation (`size=`/`stop=`/défaut), `free = true` pour arguments texte libre, body = stdin exclusivement, stdin fermé = erreur majeure, pas de rétrocompatibilité requise
- **Claude (PM/Tech Lead)** : analyse des 4 positions de délimitation, analyse granularité des limites (par action), séparation body/arguments, pragmatisme session (one-shot d'abord), synthèse de l'exercice d'alignement 004
- **Agents Claude Code** : rédaction ADR initiale, implémentation future
