# Famille : Configurations invalides

Teste le comportement de SSH Frontière face à des fichiers de configuration TOML malformés, incomplets ou incohérents. Le programme doit échouer rapidement (fail-fast) avec le code 129 et un message d'erreur clair, sans jamais exécuter de commande.

---

## SC-CFG-001 : Fichier de configuration inexistant

**Contexte** : Aucun fichier à l'emplacement configuré. Lancement avec `--config /tmp/nonexistent.toml`.

**Action** :
```
(le programme ne lit pas stdin — il échoue au démarrage)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `{"command":null,"status_code":129,"status_message":"config error: file not found","stdout":null,"stderr":null}`
- Logs : événement `config_error` avec `reason` mentionnant le chemin du fichier

**Risque couvert** : Déploiement avec chemin de config erroné — le programme ne doit pas démarrer silencieusement sans config.

---

## SC-CFG-002 : TOML syntaxiquement invalide

**Contexte** : Fichier config contenant du TOML malformé.

```toml
[global
log_file = "/var/log/ssh-frontiere/commands.json"
```

**Action** :
```
(le programme échoue au parsing TOML avant de lire stdin)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant une erreur de parsing TOML
- Logs : événement `config_error`

**Risque couvert** : Erreur d'édition manuelle du fichier de config — doit être détectée immédiatement.

---

## SC-CFG-003 : Aucun domaine défini

**Contexte** : Config avec section `[global]` valide mais aucun `[domains.*]`.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
```

**Action** :
```
(le programme échoue à la validation structurelle)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` indiquant l'absence de domaines
- Logs : événement `config_error`

**Risque couvert** : Config vide déployée par erreur — le programme ne doit pas démarrer sans aucune action utilisable.

---

## SC-CFG-004 : Domaine sans action

**Contexte** : Un domaine déclaré sans aucune action.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.orphan]
description = "Domaine sans action"
```

**Action** :
```
(le programme échoue à la validation)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant le domaine sans action
- Logs : événement `config_error`

**Risque couvert** : Config incomplète — un domaine sans action est inutile et probablement une erreur.

---

## SC-CFG-005 : Action sans champ `execute`

**Contexte** : Une action déclarée sans le champ obligatoire `execute`.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Vérification"
level = "read"
```

**Action** :
```
(le programme échoue à la validation)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant le champ `execute` manquant
- Logs : événement `config_error`

**Risque couvert** : Action mal définie — sans `execute`, l'action ne peut rien faire.

---

## SC-CFG-006 : Action sans champ `level`

**Contexte** : Une action sans le champ obligatoire `level`.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Vérification"
execute = "/usr/local/bin/check.sh"
```

**Action** :
```
(le programme échoue à la validation)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant le champ `level` manquant
- Logs : événement `config_error`

**Risque couvert** : Sans niveau RBAC explicite, impossible de savoir qui peut exécuter l'action.

---

## SC-CFG-007 : Niveau RBAC invalide dans une action

**Contexte** : Un niveau RBAC inconnu dans la configuration.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Vérification"
level = "superadmin"
execute = "/usr/local/bin/check.sh"
```

**Action** :
```
(le programme échoue à la validation)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant le niveau invalide
- Logs : événement `config_error`

**Risque couvert** : Typo dans le niveau RBAC — pourrait accidentellement rendre une action inaccessible ou trop accessible.

---

## SC-CFG-008 : Argument enum avec `default` absent de `values`

**Contexte** : Un argument enum dont la valeur par défaut n'est pas dans la liste autorisée.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.app]
description = "Application"

[domains.app.actions.deploy]
description = "Déploiement"
level = "ops"
execute = "/usr/local/bin/deploy.sh {env}"

[domains.app.actions.deploy.args]
env = { type = "enum", values = ["prod", "staging"], default = "dev" }
```

**Action** :
```
(le programme échoue à la validation)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant la valeur par défaut invalide
- Logs : événement `config_error`

**Risque couvert** : Incohérence config — un défaut inaccessible causerait des erreurs runtime difficiles à diagnostiquer.

---

## SC-CFG-009 : Placeholder dans `execute` sans argument correspondant

**Contexte** : Le champ `execute` référence un placeholder `{version}` mais aucun argument `version` n'est défini.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.app]
description = "Application"

[domains.app.actions.deploy]
description = "Déploiement"
level = "ops"
execute = "/usr/local/bin/deploy.sh {version}"
```

**Action** :
```
(le programme échoue à la validation)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant le placeholder sans argument
- Logs : événement `config_error`

**Risque couvert** : Placeholder orphelin — la commande serait exécutée avec un placeholder non résolu.

---

## SC-CFG-010 : Secret auth avec préfixe `b64:` et base64 invalide

**Contexte** : Un token d'authentification avec un secret base64 malformé.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Check"
level = "read"
execute = "/usr/local/bin/check.sh"

[auth.tokens.broken]
secret = "b64:not-valid-base64!!!"
level = "ops"
```

**Action** :
```
(le programme échoue à la validation)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant le base64 invalide
- Logs : événement `config_error`

**Risque couvert** : Secret corrompu — l'authentification serait impossible pour ce token.

---

## SC-CFG-011 : Argument enum avec liste `values` vide

**Contexte** : Un argument de type enum déclaré avec une liste de valeurs vide.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.app]
description = "Application"

[domains.app.actions.run]
description = "Exécution"
level = "ops"
execute = "/usr/local/bin/run.sh {mode}"

[domains.app.actions.run.args]
mode = { type = "enum", values = [] }
```

**Action** :
```
(le programme échoue à la validation)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant la liste de valeurs vide
- Logs : événement `config_error`

**Risque couvert** : Enum vide — aucune valeur ne serait acceptée, rendant l'action inutilisable.

---

## SC-CFG-012 : Chemin de log_file dans un répertoire inexistant

**Contexte** : Le champ `log_file` pointe vers un répertoire qui n'existe pas.

```toml
[global]
log_file = "/nonexistent/directory/commands.json"

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Check"
level = "read"
execute = "/usr/local/bin/check.sh"
```

**Action** :
```
(le programme échoue à l'ouverture du fichier de log)
```

**Attendu** :
- Code de sortie : 129
- Réponse JSON : `status_code` 129, `status_message` mentionnant l'impossibilité d'ouvrir le fichier de log
- Logs : (pas de log possible — le fichier n'est pas accessible)

**Risque couvert** : Chemin de log erroné — le programme ne doit pas fonctionner sans pouvoir journaliser.
