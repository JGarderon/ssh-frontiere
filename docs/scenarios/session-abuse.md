# Famille : Abus de sessions

Teste le comportement de SSH Frontière en mode session (`+session keepalive`) face à des séquences de commandes anormales, des tentatives d'exploitation de la persistance de connexion, et des cas limites du cycle de vie des sessions.

---

## SC-SES-001 : Session sans commande (ouverture puis fermeture)

**Contexte** : Config standard avec `--level=read`.

**Action** :
```
+ session keepalive

.
```

(Le client ouvre une session, envoie un bloc vide `.` pour terminer)

**Attendu** :
- Code de sortie : 0 (session fermée proprement)
- Pas de réponse JSON de commande (aucune commande exécutée)
- Logs : événement de session fermée sans commande

**Risque couvert** : Session vide — ouverture/fermeture sans action ne doit pas crasher ou fuiter des ressources.

---

## SC-SES-002 : Grand nombre de commandes en session

**Contexte** : Config standard avec `--level=read`. Session keepalive.

**Action** :
```
+ session keepalive

infra healthcheck
.
infra healthcheck
.
infra healthcheck
.
(... 100 fois ...)
infra healthcheck
.
.
```

**Attendu** :
- Les 100 commandes sont exécutées séquentiellement, chacune avec sa réponse `>>>`
- Code de sortie final : 0
- Logs : 100 événements `executed`

**Risque couvert** : Fuite mémoire / ressources — une session avec de nombreuses commandes ne doit pas dégrader les performances.

---

## SC-SES-003 : Commande `exit` en mode one-shot (sans session)

**Contexte** : Config standard avec `--level=read`. Pas de `+session keepalive`.

**Action** :
```

exit
.
```

**Attendu** :
- Code de sortie : 128 (la commande `exit` est un built-in de session, sans session elle est traitée comme un domaine inconnu)
- Réponse JSON : `status_code` 128
- Logs : événement `rejected`

**Risque couvert** : `exit` hors session — le comportement doit être clair et documenté.

---

## SC-SES-004 : `+session keepalive` envoyé après la phase headers

**Contexte** : Config standard avec `--level=read`. Le client tente d'activer la session après avoir déjà envoyé une commande.

**Action** :
```

infra healthcheck
.
+ session keepalive
infra healthcheck
.
```

**Attendu** :
- La première commande est exécutée en mode one-shot
- Le `+session keepalive` après la première commande est ignoré ou provoque une erreur (la phase headers est terminée)
- Le programme se ferme après la première commande (mode one-shot)
- Code de sortie : 0

**Risque couvert** : Activation tardive de session — le mode session ne peut être activé que pendant la phase headers initiale.

---

## SC-SES-005 : Ré-authentification en session pour élever le niveau

**Contexte** : Config avec deux tokens. Session keepalive. Le client s'authentifie d'abord avec un token `read`, puis élève avec un token `ops`.

```toml
[auth.tokens.viewer]
secret = "b64:dmlld2Vy"
level = "read"

[auth.tokens.operator]
secret = "b64:b3BlcmF0b3I="
level = "ops"

[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.status]
description = "Statut"
level = "read"
execute = "/usr/local/bin/status.sh"

[domains.forgejo.actions.backup]
description = "Sauvegarde"
level = "ops"
execute = "/usr/local/bin/backup.sh"
```

**Action** :
```
+ session keepalive
+ auth token=viewer proof=<valid_proof>

forgejo status
.
forgejo backup
.
+ auth token=operator proof=<valid_proof>

forgejo backup
.
.
```

**Attendu** :
- `forgejo status` : code 0 (read ≥ read)
- Premier `forgejo backup` : code 131 (read < ops)
- Après auth `operator` : niveau effectif élevé à `ops`
- Deuxième `forgejo backup` : code 0 (ops ≥ ops)
- Logs : progression du niveau effectif dans la session

**Risque couvert** : Élévation de privilèges en session — le mécanisme fonctionne correctement et le niveau monte sans jamais redescendre.

---

## SC-SES-006 : Mélange de commandes réussies et échouées en session

**Contexte** : Config avec des actions de différents niveaux. Client `--level=read`.

```toml
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.healthcheck]
level = "read"
description = "Check"
execute = "/usr/local/bin/check.sh"

[domains.infra.actions.deploy]
level = "admin"
description = "Deploy"
execute = "/usr/local/bin/deploy.sh"
```

**Action** :
```
+ session keepalive

infra healthcheck
.
infra deploy
.
infra healthcheck
.
.
```

**Attendu** :
- `healthcheck` : code 0
- `deploy` : code 131 (niveau insuffisant)
- Deuxième `healthcheck` : code 0 (la session continue malgré le rejet précédent)
- Logs : 2 `executed` + 1 `rejected`

**Risque couvert** : Résilience de session — un rejet ne doit pas terminer la session, le client peut continuer.

---

## SC-SES-007 : Commande inconnue en session

**Contexte** : Config standard. Session keepalive. Client `--level=read`.

**Action** :
```
+ session keepalive

fake-domain fake-action
.
infra healthcheck
.
.
```

**Attendu** :
- `fake-domain fake-action` : code 128 (domaine inconnu)
- `infra healthcheck` : code 0 (la session continue)
- Logs : 1 `rejected` + 1 `executed`

**Risque couvert** : Erreur de commande en session — les erreurs ne cassent pas la session.

---

## SC-SES-008 : Session avec body suivi d'une commande normale

**Contexte** : Config avec une action supportant body et une action normale. Session keepalive. Client `--level=ops`.

```toml
[domains.app]
description = "Application"

[domains.app.actions.import]
description = "Import de données"
level = "ops"
execute = "/usr/local/bin/import.sh"
max_body_size = 65536

[domains.app.actions.status]
description = "Statut"
level = "read"
execute = "/usr/local/bin/status.sh"
```

**Action** :
```
+ session keepalive
+ body

app import
ligne de données 1
ligne de données 2
.
app status
.
.
```

**Attendu** :
- `app import` : code 0, le body est passé via stdin au processus
- `app status` : code 0, exécutée normalement sans body
- Logs : 2 événements `executed`

**Risque couvert** : Transition body→normal en session — le mode body ne doit pas "fuiter" dans les commandes suivantes.

---

## SC-SES-009 : Double `+session keepalive`

**Contexte** : Config standard avec `--level=read`.

**Action** :
```
+ session keepalive
+ session keepalive

infra healthcheck
.
.
```

**Attendu** :
- Le second `+session keepalive` est ignoré ou accepté sans effet (idempotent)
- La session fonctionne normalement
- Code de sortie : 0

**Risque couvert** : Header dupliqué — l'idempotence évite des comportements inattendus.

---

## SC-SES-010 : Session avec commande `help` intercalée

**Contexte** : Config standard. Session keepalive. Client `--level=read`.

**Action** :
```
+ session keepalive

help
.
infra healthcheck
.
help infra
.
.
```

**Attendu** :
- `help` : code 0, liste des commandes accessibles
- `infra healthcheck` : code 0
- `help infra` : code 0, détails du domaine infra
- Les commandes `help` ne perturbent pas le flux de la session

**Risque couvert** : Découverte en session — les commandes built-in doivent fonctionner normalement dans le contexte d'une session.

---

## SC-SES-011 : Fermeture brutale de stdin en milieu de session

**Contexte** : Config standard. Session keepalive. Client `--level=read`.

**Action** :
```
+ session keepalive

infra healthcheck
.
infra health
(stdin fermé brutalement ici)
```

**Attendu** :
- Première commande : code 0, exécutée normalement
- Deuxième commande : incomplète, stdin fermé
- Code de sortie final : 0 ou 132/133
- Pas de crash, pas de processus zombie

**Risque couvert** : Déconnexion réseau en session — le serveur doit gérer proprement une fermeture inattendue de stdin.
