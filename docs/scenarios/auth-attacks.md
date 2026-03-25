# Famille : Attaques d'authentification

Teste la résistance de SSH Frontière aux tentatives d'authentification frauduleuses : tokens invalides, proofs forgés, rejeu de nonce, dépassement du nombre de tentatives, et contournement du challenge-response.

---

## SC-ATK-001 : Token inexistant

**Contexte** : Config avec auth activée, token `runner-ci` défini. Client avec `--level=read`.

```toml
[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
```

**Action** :
```
+ auth token=unknown-token proof=abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 131
- Réponse JSON : `status_code` 131, `status_message` indiquant un token inconnu
- Logs : événement `rejected` avec raison mentionnant le token inexistant

**Risque couvert** : Énumération de tokens — un attaquant teste des noms de tokens au hasard.

---

## SC-ATK-002 : Proof incorrect (mode simple)

**Contexte** : Config avec `challenge_nonce = false`, token `runner-ci` défini. Client avec `--level=read`.

```toml
[auth]
challenge_nonce = false

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
```

**Action** :
```
+ auth token=runner-ci proof=0000000000000000000000000000000000000000000000000000000000000000

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 131
- Réponse JSON : `status_code` 131, `status_message` indiquant un proof invalide
- Logs : événement `rejected` avec raison mentionnant l'échec d'authentification

**Risque couvert** : Proof forgé — l'attaquant connaît le nom du token mais pas le secret.

---

## SC-ATK-003 : Proof vide

**Contexte** : Config avec auth activée. Client avec `--level=read`.

**Action** :
```
+ auth token=runner-ci proof=

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 131 ou 132
- Réponse JSON : `status_code` indiquant un proof invalide ou une erreur de protocole
- Logs : événement indiquant le proof vide ou malformé

**Risque couvert** : Omission du proof — l'attaquant tente de s'authentifier sans fournir de preuve.

---

## SC-ATK-004 : Dépassement du nombre de tentatives (lockout)

**Contexte** : Config avec `max_auth_failures = 3`. Client avec `--level=read`.

```toml
[global]
max_auth_failures = 3
```

**Action** :
```
+ auth token=runner-ci proof=0000000000000000000000000000000000000000000000000000000000000001
+ auth token=runner-ci proof=0000000000000000000000000000000000000000000000000000000000000002
+ auth token=runner-ci proof=0000000000000000000000000000000000000000000000000000000000000003

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 132
- Réponse JSON : `status_code` 132, `status_message` mentionnant le lockout
- Logs : événement `auth_lockout` après la 3e tentative échouée

**Risque couvert** : Brute force en ligne — l'attaquant essaie de nombreux proofs dans une seule connexion.

---

## SC-ATK-005 : Tentative d'auth après lockout

**Contexte** : Config avec `max_auth_failures = 2`. Client avec `--level=read`.

```toml
[global]
max_auth_failures = 2
```

**Action** :
```
+ auth token=runner-ci proof=0000000000000000000000000000000000000000000000000000000000000001
+ auth token=runner-ci proof=0000000000000000000000000000000000000000000000000000000000000002
+ auth token=runner-ci proof=correct_proof_here

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 132
- Réponse JSON : `status_code` 132, la connexion est coupée après le lockout, la 3e tentative est ignorée
- Logs : événement `auth_lockout` après la 2e tentative

**Risque couvert** : Contournement du lockout — l'attaquant envoie le bon proof après le verrouillage.

---

## SC-ATK-006 : Header `+auth` malformé (champs manquants)

**Contexte** : Config avec auth activée. Client avec `--level=read`.

**Action** :
```
+ auth token=runner-ci

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 132
- Réponse JSON : `status_code` 132, `status_message` indiquant un header auth malformé
- Logs : événement indiquant l'erreur de parsing du header auth

**Risque couvert** : Header incomplet — l'attaquant omet le proof en espérant un bypass.

---

## SC-ATK-007 : Proof avec caractères non hexadécimaux

**Contexte** : Config avec auth activée. Client avec `--level=read`.

**Action** :
```
+ auth token=runner-ci proof=ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 131 ou 132
- Réponse JSON : `status_code` indiquant un proof invalide
- Logs : événement mentionnant le format de proof incorrect

**Risque couvert** : Injection dans le proof — l'attaquant envoie des données arbitraires dans le champ proof.

---

## SC-ATK-008 : Proof tronqué (longueur incorrecte)

**Contexte** : Config avec auth activée. Client avec `--level=read`.

**Action** :
```
+ auth token=runner-ci proof=abcdef01

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 131
- Réponse JSON : `status_code` 131, `status_message` indiquant un proof invalide
- Logs : événement mentionnant l'échec d'authentification

**Risque couvert** : Proof tronqué — la comparaison en temps constant ne doit pas fuiter d'information sur la longueur attendue.

---

## SC-ATK-009 : Rejeu du même nonce (mode nonce)

**Contexte** : Config avec `challenge_nonce = true`. Session keepalive. Le client s'authentifie une première fois avec succès, puis tente de réutiliser le même proof.

```toml
[auth]
challenge_nonce = true

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
```

**Action** :
```
+ session keepalive
+ auth token=runner-ci proof=<valid_proof_for_nonce_1>

infra healthcheck
.
+ auth token=runner-ci proof=<same_proof_reused>

infra healthcheck
.
```

**Attendu** :
- Première commande : succès (code 0)
- Deuxième auth : échec (le nonce a été régénéré après la première auth réussie, l'ancien proof est invalide)
- Logs : premier auth réussi, second auth échoué

**Risque couvert** : Attaque par rejeu — réutilisation d'un proof capturé sur le réseau.

---

## SC-ATK-010 : Auth sans section `[auth]` dans la config

**Contexte** : Config sans aucune section `[auth]`. Client avec `--level=read`.

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Check"
level = "read"
execute = "/usr/local/bin/check.sh"
```

**Action** :
```
+ auth token=runner-ci proof=abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789

infra check
.
```

**Attendu** :
- Code de sortie : 131 ou 132
- Réponse JSON : `status_code` indiquant que l'authentification n'est pas disponible
- Logs : événement indiquant que l'auth n'est pas configurée

**Risque couvert** : Auth non configurée — le serveur ne doit pas crasher sur un header auth inattendu.

---

## SC-ATK-011 : Multiples headers `+auth` avec tokens différents pour cumuler les niveaux

**Contexte** : Config avec deux tokens, un `read` et un `ops`. Client avec `--level=read`.

```toml
[auth.tokens.viewer]
secret = "b64:dmlld2Vy"
level = "read"

[auth.tokens.operator]
secret = "b64:b3BlcmF0b3I="
level = "ops"
```

**Action** :
```
+ auth token=viewer proof=<valid_proof_viewer>
+ auth token=operator proof=<invalid_proof>

forgejo backup-config
.
```

**Attendu** :
- Le proof invalide pour `operator` est compté comme un échec
- Le niveau effectif reste celui de `viewer` (read), pas `ops`
- Code de sortie : 131 (niveau insuffisant pour `backup-config` qui requiert `ops`)
- Logs : auth réussie pour `viewer`, auth échouée pour `operator`

**Risque couvert** : Escalade partielle — l'attaquant tente de combiner un token valide faible avec un token invalide fort.

---

## SC-ATK-012 : Header `+auth` avec champs supplémentaires injectés

**Contexte** : Config avec auth activée. Client avec `--level=read`.

**Action** :
```
+ auth token=runner-ci proof=abcdef01 level=admin extra=malicious

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 131 ou 132
- Réponse JSON : les champs supplémentaires (`level=admin`, `extra=malicious`) sont ignorés ou provoquent une erreur de protocole
- Logs : les champs injectés ne modifient pas le niveau effectif

**Risque couvert** : Injection de paramètres dans le header auth — l'attaquant tente de forcer un niveau via le header.
