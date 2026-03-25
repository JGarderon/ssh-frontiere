# Famille : Abus du protocole

Teste le comportement de SSH Frontière face à des violations du protocole d'en-têtes : préfixes invalides, lignes malformées, séquences inattendues, et flux de données anormaux. Le protocole utilise 4 préfixes (`+`, `#`, `$`, `>`) avec des règles strictes de direction et de séquencement.

---

## SC-PRO-001 : Préfixe `>` envoyé par le client

**Contexte** : Config standard avec `--level=read`. Le préfixe `>` est réservé à la direction serveur → client.

**Action** :
```
> {"status_code":0,"status_message":"injected"}

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 132
- Réponse JSON : `status_code` 132, `status_message` indiquant une erreur de protocole (préfixe invalide en entrée)
- Logs : événement `parse_error` mentionnant l'utilisation du préfixe `>` en entrée

**Risque couvert** : Usurpation de réponse — le client tente d'injecter une fausse réponse JSON.

---

## SC-PRO-002 : Ligne sans préfixe reconnu dans la phase headers

**Contexte** : Config standard avec `--level=read`.

**Action** :
```
INVALID_HEADER_LINE

infra healthcheck
.
```

**Attendu** :
- La ligne `INVALID_HEADER_LINE` est interprétée comme le début de la phase commande (fin implicite des headers), pas comme un header
- Le parseur tente de l'interpréter comme `domaine action` — `INVALID_HEADER_LINE` comme domaine sans action
- Code de sortie : 128
- Réponse JSON : `status_code` 128

**Risque couvert** : Confusion de phase — une ligne arbitraire dans la phase headers ne doit pas être ignorée silencieusement.

---

## SC-PRO-003 : Header `+session` avec valeur invalide

**Contexte** : Config standard avec `--level=read`.

**Action** :
```
+ session invalid_value

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 132
- Réponse JSON : `status_code` 132, `status_message` indiquant un header session malformé
- Logs : événement `parse_error`

**Risque couvert** : Header session malformé — seule la valeur `keepalive` est autorisée.

---

## SC-PRO-004 : Header `+` sans directive connue

**Contexte** : Config standard avec `--level=read`.

**Action** :
```
+ unknown_directive value

infra healthcheck
.
```

**Attendu** :
- Code de sortie : 132
- Réponse JSON : `status_code` 132, `status_message` indiquant une directive inconnue
- Logs : événement mentionnant la directive `unknown_directive` non reconnue

**Risque couvert** : Extension de protocole non supportée — le serveur ne doit pas ignorer silencieusement des directives inconnues.

---

## SC-PRO-005 : Ligne extrêmement longue (>64 Ko)

**Contexte** : Config standard avec `--level=read`.

**Action** :
```

infra healthcheck name=AAAAAA...(65536 caractères)...AAAAAA
.
```

**Attendu** :
- Code de sortie : 132
- Réponse JSON : `status_code` 132, `status_message` mentionnant une ligne trop longue
- Logs : événement indiquant le dépassement de la taille de ligne

**Risque couvert** : DoS par ligne géante — le parseur ne doit pas allouer de mémoire sans limite.

---

## SC-PRO-006 : Stdin fermé immédiatement (aucune donnée)

**Contexte** : Config standard avec `--level=read`.

**Action** :
```
(stdin fermé immédiatement, aucune donnée envoyée)
```

**Attendu** :
- Code de sortie : 132 ou 133
- Réponse JSON : `status_code` indiquant une erreur de protocole ou stdin fermé
- Logs : événement correspondant

**Risque couvert** : Connexion avortée — le programme ne doit pas crasher si le client ferme stdin sans rien envoyer.

---

## SC-PRO-007 : Commande sans terminateur `.`

**Contexte** : Config standard avec `--level=read`. Mode session.

**Action** :
```
+ session keepalive

infra healthcheck
(stdin fermé sans envoyer ".")
```

**Attendu** :
- Code de sortie : la commande est exécutée normalement (en mode one-shot, la fin de stdin termine la commande ; en session, la fermeture de stdin termine la session)
- Réponse JSON : résultat de la commande
- Logs : événement `executed`

**Risque couvert** : Absence de terminateur — le protocole doit gérer proprement la fin de stdin comme fin implicite.

---

## SC-PRO-008 : Multiples lignes vides entre headers et commande

**Contexte** : Config standard avec `--level=read`.

**Action** :
```



infra healthcheck
.
```

(Trois lignes vides avant la commande)

**Attendu** :
- La première ligne vide termine les headers
- Les lignes vides suivantes sont interprétées dans la phase commande
- La commande `infra healthcheck` est exécutée
- Code de sortie : 0

**Risque couvert** : Lignes vides parasites — le parseur doit être tolérant aux lignes vides excédentaires.

---

## SC-PRO-009 : Header `+body` avec taille négative

**Contexte** : Config avec une action supportant body. Client avec `--level=ops`.

**Action** :
```
+ body size=-1

app import
contenu du body
.
```

**Attendu** :
- Code de sortie : 132
- Réponse JSON : `status_code` 132, `status_message` indiquant une taille de body invalide
- Logs : événement `parse_error`

**Risque couvert** : Taille de body négative — pourrait causer un underflow ou un comportement indéfini.

---

## SC-PRO-010 : Header `+body` avec taille dépassant `max_body_size`

**Contexte** : Config avec une action ayant `max_body_size = 1024`. Client avec `--level=ops`.

```toml
[domains.app.actions.import]
description = "Import"
level = "ops"
execute = "/usr/local/bin/import.sh"
max_body_size = 1024
```

**Action** :
```
+ body size=999999

app import
(999999 bytes de données)
.
```

**Attendu** :
- Code de sortie : 128 ou 132
- Réponse JSON : `status_code` indiquant un rejet — le body est rejeté AVANT lecture (pas de lecture puis troncature)
- Logs : événement mentionnant le dépassement de `max_body_size`

**Risque couvert** : DoS par body géant — le serveur doit refuser avant de consommer la mémoire.

---

## SC-PRO-011 : Header `+body` avec délimiteur stop jamais envoyé

**Contexte** : Config avec une action supportant body. Client avec `--level=ops`.

**Action** :
```
+ body stop="---END---"

app import
ligne 1
ligne 2
ligne 3
(stdin fermé sans envoyer "---END---")
```

**Attendu** :
- Code de sortie : 133
- Réponse JSON : `status_code` 133, `status_message` indiquant que le body n'a pas été terminé
- Logs : événement mentionnant la fermeture de stdin avant le délimiteur

**Risque couvert** : Body incomplet — le client interrompt la transmission avant la fin.

---

## SC-PRO-012 : Préfixe `$` (ancienne syntaxe) pour la commande

**Contexte** : Config standard avec `--level=read`.

**Action** :
```

$ infra healthcheck
.
```

**Attendu** :
- Le préfixe `$` est reconnu comme un préfixe de commande valide (compatibilité)
- Code de sortie : 0
- Réponse JSON : résultat de l'exécution

**Risque couvert** : Compatibilité protocole — les clients utilisant l'ancien format `$ commande` doivent rester fonctionnels.

---

## SC-PRO-013 : Header `+body` sur une action sans `max_body_size`

**Contexte** : Config avec une action qui n'a pas de `max_body_size` défini. Client avec `--level=read`.

```toml
[domains.infra.actions.healthcheck]
description = "Check"
level = "read"
execute = "/usr/local/bin/healthcheck.sh"
```

**Action** :
```
+ body

infra healthcheck
contenu inattendu
.
```

**Attendu** :
- Le body est accepté avec la taille par défaut (65536 bytes) ou traité selon le comportement défini
- Code de sortie : dépend du comportement (0 si body passé au processus, ou erreur si non supporté)
- Logs : événement correspondant

**Risque couvert** : Body non attendu par l'action — le serveur doit gérer le cas où un body est envoyé pour une action qui ne le prévoit pas.

---

## SC-PRO-014 : Envoi de données binaires sur stdin

**Contexte** : Config standard avec `--level=read`.

**Action** :
```
(octets binaires aléatoires : \x00\x01\xff\xfe sur stdin)
```

**Attendu** :
- Code de sortie : 132 ou 128
- Réponse JSON : `status_code` indiquant une erreur de protocole ou de parsing
- Logs : événement indiquant un contenu non-UTF-8 ou non parseable

**Risque couvert** : Injection binaire — le protocole est textuel (lignes), des octets non-UTF-8 ne doivent pas crasher le parseur.
