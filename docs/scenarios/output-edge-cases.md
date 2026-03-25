# Famille : Cas limites de sortie

Teste le comportement de SSH Frontière pour la production de réponses JSON, le streaming de stdout/stderr, les limites de taille de sortie, et les cas limites de formatage. Chaque commande doit produire exactement une réponse JSON `>>>` bien formée, quel que soit le résultat.

---

## SC-OUT-001 : Réponse JSON pour commande réussie

**Contexte** : Config standard avec `--level=read`. L'action produit de la sortie.

```toml
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.healthcheck]
description = "Check"
level = "read"
execute = "/usr/local/bin/healthcheck.sh"
```

(Le script `healthcheck.sh` fait `echo "OK"`)

**Action** :
```

infra healthcheck
.
```

**Attendu** :
- Lignes de streaming : `>> OK`
- Réponse JSON : `>>> {"command":"infra healthcheck","status_code":0,"status_message":"executed","stdout":null,"stderr":null}`
- `stdout` est `null` car le contenu a été streamé via `>>`
- Code de sortie : 0

**Risque couvert** : Format de réponse standard — la réponse JSON doit contenir exactement 5 champs avec les types corrects.

---

## SC-OUT-002 : Réponse JSON pour commande refusée

**Contexte** : Config avec une action `admin`. Client `--level=read`.

**Action** :
```

infra deploy
.
```

**Attendu** :
- Pas de lignes `>>` ou `>>!` (la commande n'est pas exécutée)
- Réponse JSON : `>>> {"command":"infra deploy","status_code":131,"status_message":"rejected: insufficient level","stdout":null,"stderr":null}`
- `stdout` et `stderr` sont `null` (commande non exécutée)
- Code de sortie : 131

**Risque couvert** : Format de réponse pour rejet — la réponse doit être JSON valide même en cas de rejet.

---

## SC-OUT-003 : Commande avec sortie stderr

**Contexte** : Config standard. Le processus enfant écrit sur stderr.

(Le script fait `echo "error message" >&2; exit 1`)

**Action** :
```

infra failing-check
.
```

**Attendu** :
- Lignes de streaming : `>>! error message`
- Réponse JSON : `>>> {"command":"infra failing-check","status_code":1,"status_message":"executed","stdout":null,"stderr":null}`
- Le code de sortie du processus enfant (1) est transmis tel quel
- Code de sortie : 1

**Risque couvert** : Transmission de stderr — les erreurs du processus enfant doivent être visibles via `>>!`.

---

## SC-OUT-004 : Commande avec sortie mélangée stdout et stderr

**Contexte** : Config standard. Le processus enfant écrit alternativement sur stdout et stderr.

(Le script fait : `echo "out1"; echo "err1" >&2; echo "out2"; echo "err2" >&2`)

**Action** :
```

infra mixed-output
.
```

**Attendu** :
- Lignes de streaming entrelacées : `>> out1`, `>>! err1`, `>> out2`, `>>! err2` (l'ordre exact peut varier selon le buffering)
- Réponse JSON : `status_code` 0, `stdout` null, `stderr` null
- Code de sortie : 0

**Risque couvert** : Entrelacement stdout/stderr — les deux flux doivent être correctement préfixés et distinguables.

---

## SC-OUT-005 : Commande sans aucune sortie

**Contexte** : Config standard. Le processus enfant ne produit aucune sortie.

(Le script fait `exit 0` sans rien écrire)

**Action** :
```

infra silent-check
.
```

**Attendu** :
- Pas de lignes `>>` ni `>>!`
- Réponse JSON : `>>> {"command":"infra silent-check","status_code":0,"status_message":"executed","stdout":null,"stderr":null}`
- Code de sortie : 0

**Risque couvert** : Sortie vide — l'absence de sortie ne doit pas bloquer la génération de la réponse JSON.

---

## SC-OUT-006 : Bannière serveur complète

**Contexte** : Config avec auth activée et `challenge_nonce = true`.

**Action** :
```
(le client se connecte et lit la bannière)
```

**Attendu** :
- Première ligne : `#> ssh-frontiere <version>` (version du binaire)
- Ligne capabilities : `+> capabilities rbac, session, help, body` (selon la config)
- Ligne challenge : `+> challenge nonce=<32_hex_chars>` (si auth tokens configurés et nonce activé)
- Ligne d'aide : `#> ...` (texte d'aide)
- Toutes les lignes utilisent les préfixes corrects (`#>` et `+>`)

**Risque couvert** : Format de bannière — la bannière est le premier contact du protocole, elle doit être complète et correctement formatée.

---

## SC-OUT-007 : Réponse JSON avec caractères spéciaux dans stdout

**Contexte** : Config standard. Le processus enfant produit du JSON, des guillemets, et des caractères d'échappement.

(Le script fait `echo '{"key": "value with \"quotes\" and \nnewline"}'`)

**Action** :
```

infra json-output
.
```

**Attendu** :
- Lignes de streaming : `>> {"key": "value with \"quotes\" and \nnewline"}`
- Réponse JSON finale : valide et bien échappée, `stdout` null (streamé)
- Les caractères spéciaux dans la sortie ne cassent PAS le JSON de la réponse `>>>`

**Risque couvert** : Échappement JSON — la sortie du processus enfant peut contenir n'importe quoi, la réponse `>>>` doit rester du JSON valide.

---

## SC-OUT-008 : Argument sensible masqué dans les logs

**Contexte** : Config avec `mask_sensitive = true` et une action avec un argument sensible.

```toml
[global]
mask_sensitive = true

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.set-password]
description = "Set password"
level = "admin"
execute = "/usr/local/bin/set-password.sh {password}"

[domains.infra.actions.set-password.args]
password = { type = "string", sensitive = true }
```

**Action** (client `--level=admin`) :
```

infra set-password password=SuperSecret123
.
```

**Attendu** :
- Réponse JSON client : la commande est exécutée normalement
- Logs : l'argument `password` est remplacé par `sha256:<hash_hex>` dans les logs JSON
- La valeur réelle `SuperSecret123` N'apparaît PAS dans les logs
- Code de sortie : 0

**Risque couvert** : Fuite de secrets dans les logs — les arguments sensibles doivent être masqués pour la conformité et la sécurité.

---

## SC-OUT-009 : Valeurs par défaut appliquées et tracées dans les logs

**Contexte** : Config avec une action ayant des arguments avec valeurs par défaut.

```toml
[domains.app]
description = "Application"

[domains.app.actions.deploy]
description = "Deploy"
level = "ops"
execute = "/usr/local/bin/deploy.sh {env} {verbose}"

[domains.app.actions.deploy.args]
env = { type = "enum", values = ["prod", "staging"], default = "staging" }
verbose = { type = "enum", values = ["true", "false"], default = "false" }
```

**Action** (client `--level=ops`) :
```

app deploy
.
```

**Attendu** :
- Code de sortie : 0
- La commande est exécutée avec `env=staging` et `verbose=false`
- Logs : événement `executed` avec `args` contenant les valeurs par défaut ET `defaults_applied` listant les arguments dont le défaut a été utilisé

**Risque couvert** : Traçabilité des défauts — quand des valeurs par défaut sont appliquées, les logs doivent le montrer pour le debugging.

---

## SC-OUT-010 : Réponse JSON pour erreur de protocole

**Contexte** : Config standard avec `--level=read`.

**Action** :
```
+ invalid_directive

infra healthcheck
.
```

**Attendu** :
- Réponse JSON : `>>> {"command":null,"status_code":132,"status_message":"protocol error: ...","stdout":null,"stderr":null}`
- Le champ `command` est `null` (la commande n'a pas pu être parsée)
- Code de sortie : 132

**Risque couvert** : Format de réponse pour erreur protocole — même les erreurs de protocole doivent produire une réponse JSON parseable.

---

## SC-OUT-011 : Argument `key=value=with=equals` (split sur le premier `=`)

**Contexte** : Config avec une action acceptant un argument string.

```toml
[domains.app]
description = "Application"

[domains.app.actions.configure]
description = "Configuration"
level = "ops"
execute = "/usr/local/bin/configure.sh {setting}"

[domains.app.actions.configure.args]
setting = { type = "string" }
```

**Action** (client `--level=ops`) :
```

app configure setting=key=value=with=equals
.
```

**Attendu** :
- Code de sortie : 0
- La valeur de l'argument `setting` est `key=value=with=equals` (split sur le premier `=` uniquement)
- Logs : événement `executed` avec `args.setting` = `key=value=with=equals`

**Risque couvert** : Parsing d'arguments avec `=` dans la valeur — le split doit être fait sur le premier `=` uniquement.

---

## SC-OUT-012 : Argument avec valeur vide (`key=`)

**Contexte** : Config avec une action acceptant un argument string.

**Action** (client `--level=ops`) :
```

app configure setting=
.
```

**Attendu** :
- Code de sortie : 0
- La valeur de l'argument `setting` est une chaîne vide `""`
- Logs : événement `executed` avec `args.setting` = `""`

**Risque couvert** : Valeur vide — `key=` est une syntaxe valide qui produit une chaîne vide.

---

## SC-OUT-013 : Argument dupliqué dans une commande

**Contexte** : Config avec une action acceptant un argument.

**Action** (client `--level=ops`) :
```

app deploy env=prod env=staging
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant un argument dupliqué
- Logs : événement `rejected`

**Risque couvert** : Argument dupliqué — le parseur ne doit pas silencieusement prendre le premier ou le dernier, il doit rejeter.

---

## SC-OUT-014 : Argument inconnu dans une commande

**Contexte** : Config avec une action ayant un seul argument défini.

**Action** (client `--level=ops`) :
```

app deploy env=prod unknown_arg=value
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant un argument inconnu
- Logs : événement `rejected`

**Risque couvert** : Argument non déclaré — le parseur ne doit pas ignorer les arguments non définis dans la config.
