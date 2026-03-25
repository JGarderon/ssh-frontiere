# Famille : Tentatives d'injection

Teste la résistance de SSH Frontière aux tentatives d'injection de commandes, de contournement du parseur grammatical, et d'exploitation via des arguments malveillants. Le parseur grammatical (`domaine action [key=value...]`) et l'exécution directe via `std::process::Command` rendent l'injection structurellement impossible, mais ces scénarios vérifient que cette garantie tient dans tous les cas limites.

---

## SC-INJ-001 : Pipe dans une commande

**Contexte** : Config standard avec `--level=read`.

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

infra healthcheck | cat /etc/passwd
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant une commande invalide (la grammaire n'accepte pas `|`)
- Logs : événement `rejected` ou `parse_error`

**Risque couvert** : Injection de pipe — tentative classique de chaîner une commande arbitraire.

---

## SC-INJ-002 : Point-virgule pour chaîner une commande

**Contexte** : Config standard avec `--level=read`.

**Action** :
```

infra healthcheck; rm -rf /
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant une commande invalide
- Logs : événement `rejected` ou `parse_error`

**Risque couvert** : Chaînage par `;` — le parseur grammatical rejette toute syntaxe shell.

---

## SC-INJ-003 : Double ampersand pour chaîner

**Contexte** : Config standard avec `--level=read`.

**Action** :
```

infra healthcheck && cat /etc/shadow
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant une commande invalide
- Logs : événement `rejected` ou `parse_error`

**Risque couvert** : Chaînage conditionnel `&&` — variante d'injection par opérateur shell.

---

## SC-INJ-004 : Substitution de commande dans un argument

**Contexte** : Config avec une action acceptant un argument string.

```toml
[domains.app]
description = "Application"

[domains.app.actions.greet]
description = "Salutation"
level = "read"
execute = "/usr/local/bin/greet.sh {name}"

[domains.app.actions.greet.args]
name = { type = "string" }
```

**Action** :
```

app greet name=$(cat /etc/passwd)
.
```

**Attendu** :
- Code de sortie : 0 (la commande est exécutée)
- Réponse JSON : `status_code` 0 — la valeur littérale `$(cat /etc/passwd)` est passée comme argument textuel à `greet.sh`, PAS interprétée comme substitution
- Logs : événement `executed` avec l'argument littéral (pas le contenu de /etc/passwd)

**Risque couvert** : Substitution de commande — `std::process::Command` n'invoque pas de shell, donc `$(...)` est du texte brut.

---

## SC-INJ-005 : Backtick substitution dans un argument

**Contexte** : Config avec une action acceptant un argument string.

**Action** :
```

app greet name=`id`
.
```

**Attendu** :
- Code de sortie : 0
- Réponse JSON : `status_code` 0 — la valeur littérale `` `id` `` est passée comme texte brut
- Logs : événement `executed` avec l'argument littéral

**Risque couvert** : Backtick substitution — variante historique de `$(...)`, doit être traitée comme du texte.

---

## SC-INJ-006 : Redirection dans une commande

**Contexte** : Config standard avec `--level=read`.

**Action** :
```

infra healthcheck > /tmp/output.txt
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant une commande invalide (le `>` après la commande n'est pas un argument nommé `key=value`)
- Logs : événement `rejected` ou `parse_error`

**Risque couvert** : Redirection de sortie — tentative d'écriture dans un fichier arbitraire.

---

## SC-INJ-007 : Caractères spéciaux entre guillemets (valides)

**Contexte** : Config avec une action acceptant un argument string.

```toml
[domains.app]
description = "Application"

[domains.app.actions.greet]
description = "Salutation"
level = "read"
execute = "/usr/local/bin/greet.sh {msg}"

[domains.app.actions.greet.args]
msg = { type = "string" }
```

**Action** :
```

app greet msg="hello | world; rm -rf / && echo pwned"
.
```

**Attendu** :
- Code de sortie : 0
- Réponse JSON : `status_code` 0 — la chaîne entière est un argument valide, les caractères spéciaux sont du contenu
- Logs : événement `executed` avec la valeur complète de l'argument

**Risque couvert** : Faux positif — le parseur grammatical ne doit PAS rejeter du contenu légitime entre guillemets. C'est le principe fondamental : parseur grammatical, pas liste noire.

---

## SC-INJ-008 : Argument avec saut de ligne échappé

**Contexte** : Config avec une action acceptant un argument string.

**Action** :
```

app greet msg="line1\nline2"
.
```

**Attendu** :
- Code de sortie : 0
- Réponse JSON : `status_code` 0 — la valeur `line1\nline2` est passée littéralement (le `\n` n'est pas interprété comme saut de ligne)
- Logs : événement `executed`

**Risque couvert** : Injection de saut de ligne — le protocole est line-based, un `\n` dans un argument ne doit pas casser le parsing.

---

## SC-INJ-009 : Domaine inexistant (fuzzing de commande)

**Contexte** : Config standard avec `--level=read`.

**Action** :
```

../../etc/passwd cat
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant un domaine inconnu
- Logs : événement `rejected`

**Risque couvert** : Path traversal dans le nom de domaine — le domaine est une clé de lookup, pas un chemin fichier.

---

## SC-INJ-010 : Argument avec valeur très longue

**Contexte** : Config avec une action acceptant un argument string (max 256 caractères).

**Action** :
```

app greet name=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
.
```

(260 caractères de 'A')

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant un argument trop long
- Logs : événement `rejected` avec raison mentionnant la longueur

**Risque couvert** : Buffer overflow / DoS via argument géant — la limite de 256 caractères pour les arguments string doit être appliquée.

---

## SC-INJ-011 : Commande avec uniquement un domaine (action manquante)

**Contexte** : Config standard avec `--level=read`.

**Action** :
```

infra
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant une action manquante
- Logs : événement `rejected` ou `parse_error`

**Risque couvert** : Commande tronquée — le parseur ne doit pas interpréter un domaine seul comme une commande valide.

---

## SC-INJ-012 : Argument positionnel (non nommé)

**Contexte** : Config avec une action acceptant un argument.

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

**Action** :
```

app deploy prod
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant un argument positionnel non autorisé (la syntaxe requise est `key=value`)
- Logs : événement `rejected`

**Risque couvert** : Arguments positionnels — rejetés depuis Phase 5.5, la syntaxe est exclusivement `key=value`.

---

## SC-INJ-013 : Variable d'environnement dans un argument

**Contexte** : Config avec une action acceptant un argument string.

**Action** :
```

app greet name=$HOME
.
```

**Attendu** :
- Code de sortie : 0
- Réponse JSON : `status_code` 0 — la valeur littérale `$HOME` est passée, PAS la valeur de la variable d'environnement (pas de shell pour l'interpréter)
- Logs : événement `executed` avec la valeur littérale `$HOME`

**Risque couvert** : Expansion de variables — `std::process::Command` avec `env_clear()` ne passe pas les variables d'environnement et n'interprète pas `$VAR`.

---

## SC-INJ-014 : Ligne vide comme commande

**Contexte** : Config standard avec `--level=read`.

**Action** :
```

.
```

(Le client envoie une ligne vide pour terminer les headers, puis `.` pour terminer le bloc commande — pas de commande envoyée)

**Attendu** :
- Code de sortie : 132 ou fermeture propre de la session
- Réponse JSON : `status_code` 132 ou fin de session sans exécution
- Logs : événement correspondant

**Risque couvert** : Commande vide — le serveur ne doit pas crasher ni exécuter quoi que ce soit.
