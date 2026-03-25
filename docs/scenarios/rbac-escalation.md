# Famille : Escalade RBAC

Teste la résistance du système de contrôle d'accès RBAC (vertical par niveaux `read < ops < admin`, horizontal par tags) aux tentatives d'escalade de privilèges : accès à des actions au-dessus de son niveau, contournement des tags de visibilité, et abus des mécanismes d'élévation.

---

## SC-RBA-001 : Accès à une action `ops` avec niveau `read`

**Contexte** : Config avec une action de niveau `ops`. Client avec `--level=read`.

```toml
[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.backup-config]
description = "Sauvegarde"
level = "ops"
execute = "/usr/local/bin/backup.sh"
```

**Action** :
```

forgejo backup-config
.
```

**Attendu** :
- Code de sortie : 131
- Réponse JSON : `status_code` 131, `status_message` indiquant un niveau insuffisant
- Logs : événement `rejected` avec raison `insufficient level`, niveau effectif `read`, niveau requis `ops`

**Risque couvert** : Escalade verticale basique — un client `read` ne doit pas pouvoir exécuter une action `ops`.

---

## SC-RBA-002 : Accès à une action `admin` avec niveau `ops`

**Contexte** : Config avec une action de niveau `admin`. Client avec `--level=ops`.

```toml
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.set-password]
description = "Changement de mot de passe"
level = "admin"
execute = "/usr/local/bin/set-password.sh {password}"

[domains.infra.actions.set-password.args]
password = { type = "string", sensitive = true }
```

**Action** :
```

infra set-password password=secret123
.
```

**Attendu** :
- Code de sortie : 131
- Réponse JSON : `status_code` 131, `status_message` indiquant un niveau insuffisant
- Logs : événement `rejected`, l'argument sensible `password` doit être masqué dans les logs

**Risque couvert** : Escalade verticale ops→admin — les opérations administratives sont protégées même pour les opérateurs.

---

## SC-RBA-003 : Action protégée par tag, client sans tags

**Contexte** : Config avec une action taguée `forgejo`. Client sans token (pas de tags).

```toml
[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.status]
description = "Statut"
level = "read"
tags = ["forgejo"]
execute = "/usr/local/bin/forgejo-status.sh"
```

**Action** :
```

forgejo status
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant que l'action n'est pas accessible (pas de tag correspondant)
- Logs : événement `rejected` avec raison mentionnant les tags

**Risque couvert** : Filtrage horizontal — un client sans tags ne voit que les actions publiques (sans tags).

---

## SC-RBA-004 : Action protégée par tag, client avec mauvais tag

**Contexte** : Config avec une action taguée `mastodon`. Client authentifié avec un token ayant le tag `forgejo`.

```toml
[domains.mastodon]
description = "Mastodon"

[domains.mastodon.actions.status]
description = "Statut Mastodon"
level = "read"
tags = ["mastodon"]
execute = "/usr/local/bin/mastodon-status.sh"

[auth.tokens.forgejo-runner]
secret = "b64:Zm9yZ2Vqby1ydW5uZXI="
level = "read"
tags = ["forgejo"]
```

**Action** :
```
+ auth token=forgejo-runner proof=<valid_proof>

mastodon status
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant que l'action n'est pas accessible
- Logs : événement `rejected`, `effective_tags` = `["forgejo"]`, `action_tags` = `["mastodon"]`

**Risque couvert** : Isolation horizontale — le tag `forgejo` ne donne pas accès aux actions taguées `mastodon`.

---

## SC-RBA-005 : Action sans tags (publique) accessible à tous

**Contexte** : Config avec une action sans tags et un client authentifié avec des tags.

```toml
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.healthcheck]
description = "Check santé"
level = "read"
execute = "/usr/local/bin/healthcheck.sh"
```

**Action** :
```

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 0
- Réponse JSON : `status_code` 0, la commande est exécutée
- Logs : événement `executed`

**Risque couvert** : Vérification positive — les actions sans tags sont publiques et accessibles à tous les niveaux suffisants.

---

## SC-RBA-006 : `help` ne montre que les actions accessibles au niveau du client

**Contexte** : Config avec des actions de différents niveaux. Client avec `--level=read`.

```toml
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.healthcheck]
description = "Check santé"
level = "read"
execute = "/usr/local/bin/healthcheck.sh"

[domains.infra.actions.deploy]
description = "Déploiement"
level = "admin"
execute = "/usr/local/bin/deploy.sh"
```

**Action** :
```

help
.
```

**Attendu** :
- Code de sortie : 0
- Réponse JSON : le `stdout` (ou les lignes `>>`) ne mentionne que `healthcheck`, PAS `deploy`
- Logs : événement `discovery`

**Risque couvert** : Fuite d'information — un client `read` ne doit pas connaître l'existence des actions `admin`.

---

## SC-RBA-007 : `help` ne montre pas les actions filtrées par tags

**Contexte** : Config avec une action taguée. Client sans token (pas de tags).

```toml
[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.backup]
description = "Sauvegarde"
level = "read"
tags = ["forgejo"]
execute = "/usr/local/bin/backup.sh"

[domains.forgejo.actions.status]
description = "Statut public"
level = "read"
execute = "/usr/local/bin/status.sh"
```

**Action** :
```

help forgejo
.
```

**Attendu** :
- Code de sortie : 0
- Réponse : seule l'action `status` (sans tags) est visible, PAS `backup` (taguée `forgejo`)
- Logs : événement `discovery`

**Risque couvert** : Fuite d'information horizontale — les actions taguées sont invisibles aux clients sans le tag correspondant.

---

## SC-RBA-008 : Cumul de tags en session (élévation horizontale)

**Contexte** : Config avec deux tokens ayant des tags différents. Session keepalive.

```toml
[auth.tokens.forgejo-runner]
secret = "b64:Zm9yZ2Vqby1ydW5uZXI="
level = "read"
tags = ["forgejo"]

[auth.tokens.mastodon-monitor]
secret = "b64:bWFzdG9kb24tbW9uaXRvcg=="
level = "read"
tags = ["mastodon"]

[domains.mastodon]
description = "Mastodon"

[domains.mastodon.actions.status]
description = "Statut"
level = "read"
tags = ["mastodon"]
execute = "/usr/local/bin/mastodon-status.sh"
```

**Action** :
```
+ session keepalive
+ auth token=forgejo-runner proof=<valid_proof>

mastodon status
.
+ auth token=mastodon-monitor proof=<valid_proof>

mastodon status
.
```

**Attendu** :
- Première commande `mastodon status` : code 128 (refusé — tags `["forgejo"]` ne matchent pas `["mastodon"]`)
- Après second `+auth` : tags effectifs = `["forgejo", "mastodon"]` (union cumulative)
- Deuxième commande `mastodon status` : code 0 (accepté)
- Logs : premier rejet avec tags incompatibles, second succès avec tags cumulés

**Risque couvert** : Cumul de tags en session — le comportement est by-design (irréversible dans la session), mais doit être vérifié.

---

## SC-RBA-009 : Impossibilité de dé-escalader le niveau en session

**Contexte** : Config avec un token `admin`. Session keepalive. Le client s'authentifie en admin puis tente de redescendre.

```toml
[auth.tokens.admin-key]
secret = "b64:YWRtaW4ta2V5"
level = "admin"

[auth.tokens.viewer]
secret = "b64:dmlld2Vy"
level = "read"
```

**Action** :
```
+ session keepalive
+ auth token=admin-key proof=<valid_proof>

infra set-password password=test
.
+ auth token=viewer proof=<valid_proof>

infra set-password password=test2
.
```

**Attendu** :
- Première commande : succès (niveau admin)
- Après auth avec `viewer` (read) : le niveau effectif reste `admin` (max de tous les niveaux vus)
- Deuxième commande : succès (le niveau ne descend pas)
- Logs : les deux commandes sont exécutées en `admin`

**Risque couvert** : Dé-escalade impossible — le niveau effectif ne peut que monter, jamais descendre, pendant une session.

---

## SC-RBA-010 : Argument enum avec valeur hors liste

**Contexte** : Config avec une action ayant un argument enum.

```toml
[domains.app]
description = "Application"

[domains.app.actions.deploy]
description = "Déploiement"
level = "ops"
execute = "/usr/local/bin/deploy.sh {env}"

[domains.app.actions.deploy.args]
env = { type = "enum", values = ["prod", "staging"] }
```

**Action** (client avec `--level=ops`) :
```

app deploy env=development
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant une valeur d'argument invalide
- Logs : événement `rejected` mentionnant la valeur `development` non dans `["prod", "staging"]`

**Risque couvert** : Contournement de la validation d'arguments — l'attaquant tente d'injecter une valeur non autorisée.

---

## SC-RBA-011 : Commande `list` filtrée par niveau et tags

**Contexte** : Config avec des actions de différents niveaux et tags. Client avec `--level=read` et token avec tag `forgejo`.

**Action** :
```
+ auth token=forgejo-runner proof=<valid_proof>

list
.
```

**Attendu** :
- Code de sortie : 0
- Réponse JSON : la liste ne contient que les actions de niveau ≤ `read` ET (sans tags OU avec tag `forgejo`)
- Logs : événement `discovery`

**Risque couvert** : Fuite d'information via `list` — la commande de découverte doit respecter le même filtrage RBAC.

---

## SC-RBA-012 : Argument obligatoire manquant

**Contexte** : Config avec une action ayant un argument obligatoire (sans default).

```toml
[domains.app]
description = "Application"

[domains.app.actions.deploy]
description = "Déploiement"
level = "ops"
execute = "/usr/local/bin/deploy.sh {env}"

[domains.app.actions.deploy.args]
env = { type = "enum", values = ["prod", "staging"] }
```

**Action** (client avec `--level=ops`) :
```

app deploy
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant l'argument obligatoire manquant
- Logs : événement `rejected`

**Risque couvert** : Argument obligatoire omis — le programme ne doit pas exécuter une commande avec des arguments manquants.
