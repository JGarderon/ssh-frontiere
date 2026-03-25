# Famille : Timeouts et pannes

Teste le comportement de SSH Frontière face aux dépassements de temps (commande, session), aux processus enfants qui ne terminent pas, aux crashs de processus, et aux situations de charge anormale. Le programme doit toujours répondre proprement, tuer les processus en dépassement, et ne jamais rester bloqué indéfiniment.

---

## SC-TMO-001 : Commande dépassant le timeout

**Contexte** : Config avec un timeout court pour une action.

```toml
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.slow-check]
description = "Check lent"
level = "read"
timeout = 2
execute = "/usr/local/bin/slow-check.sh"
```

**Action** :
```

infra slow-check
.
```

(Le script `slow-check.sh` fait `sleep 60`)

**Attendu** :
- Code de sortie : 130
- Réponse JSON : `status_code` 130, `status_message` mentionnant le timeout
- Logs : événement `timeout` avec `duration_ms` ≈ 2000
- Le processus enfant est tué (SIGTERM puis SIGKILL si nécessaire)

**Risque couvert** : DoS par commande lente — le timeout empêche un processus de bloquer indéfiniment la connexion.

---

## SC-TMO-002 : Timeout par défaut (`default_timeout`)

**Contexte** : Config avec un `default_timeout` global. Action sans timeout propre.

```toml
[global]
default_timeout = 3

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Check"
level = "read"
execute = "/usr/local/bin/slow-check.sh"
```

**Action** :
```

infra check
.
```

(Le script fait `sleep 60`)

**Attendu** :
- Code de sortie : 130
- Réponse JSON : `status_code` 130, `status_message` mentionnant le timeout
- Logs : événement `timeout` avec `duration_ms` ≈ 3000

**Risque couvert** : Timeout global — vérifie que `default_timeout` s'applique quand l'action ne définit pas son propre timeout.

---

## SC-TMO-003 : Timeout de session

**Contexte** : Config avec `timeout_session = 5`. Session keepalive.

```toml
[global]
timeout_session = 5
```

**Action** :
```
+ session keepalive

infra healthcheck
.
(le client ne fait rien pendant 6 secondes)
infra healthcheck
.
```

**Attendu** :
- Première commande : code 0, exécutée normalement
- Après 5 secondes d'inactivité : la session est fermée par le serveur
- Deuxième commande : jamais reçue (connexion fermée)
- Logs : événement indiquant la fermeture de session par timeout

**Risque couvert** : Session zombie — une session inactive ne doit pas rester ouverte indéfiniment et consommer des ressources.

---

## SC-TMO-004 : Processus enfant qui écrit infiniment sur stdout

**Contexte** : Config avec `max_stream_bytes` défini. Le processus enfant produit une sortie infinie.

```toml
[global]
max_stream_bytes = 1024

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.noisy]
description = "Sortie volumineuse"
level = "read"
timeout = 10
execute = "/usr/local/bin/noisy.sh"
```

**Action** :
```

infra noisy
.
```

(Le script fait `yes "output line"` — sortie infinie)

**Attendu** :
- Le streaming s'arrête à `max_stream_bytes` (1024 bytes)
- Un message de troncature est émis : `>>! ssh-frontiere: output truncated`
- Code de sortie : 0 ou 130 (selon si le timeout est atteint avant la limite)
- Logs : événement `executed` ou `timeout`

**Risque couvert** : DoS par sortie volumineuse — le serveur doit tronquer la sortie pour éviter d'épuiser la mémoire ou la bande passante.

---

## SC-TMO-005 : Processus enfant qui ignore SIGTERM

**Contexte** : Config avec un timeout court. Le processus enfant intercepte SIGTERM et continue.

```toml
[domains.infra.actions.stubborn]
description = "Processus têtu"
level = "read"
timeout = 2
execute = "/usr/local/bin/stubborn.sh"
```

**Action** :
```

infra stubborn
.
```

(Le script intercepte SIGTERM : `trap '' TERM; sleep 60`)

**Attendu** :
- Après le timeout (2s) : SIGTERM envoyé au process group
- Après un bref délai : SIGKILL envoyé au process group (non interceptable)
- Code de sortie : 130
- Réponse JSON : `status_code` 130
- Le processus est effectivement tué

**Risque couvert** : Processus récalcitrant — le SIGKILL garantit que le processus est toujours terminé.

---

## SC-TMO-006 : Processus enfant qui se termine avec un signal

**Contexte** : Config standard. Le processus enfant reçoit un signal externe (ex: SIGSEGV).

```toml
[domains.infra.actions.crasher]
description = "Processus qui crashe"
level = "read"
execute = "/usr/local/bin/crasher.sh"
```

**Action** :
```

infra crasher
.
```

(Le script se termine par un SIGSEGV : `kill -SEGV $$`)

**Attendu** :
- Code de sortie : code enfant (139 = 128 + 11 pour SIGSEGV, ou autre code selon la convention)
- Réponse JSON : `status_code` reflétant le signal reçu, `status_message` mentionnant le signal
- Logs : événement `executed` avec le code de sortie du signal

**Risque couvert** : Crash de processus enfant — le programme doit rapporter proprement le signal sans crasher lui-même.

---

## SC-TMO-007 : Exécutable introuvable

**Contexte** : Config avec un chemin d'exécutable qui n'existe pas.

```toml
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.ghost]
description = "Exécutable fantôme"
level = "read"
execute = "/usr/local/bin/nonexistent-script.sh"
```

**Action** :
```

infra ghost
.
```

**Attendu** :
- Code de sortie : 128
- Réponse JSON : `status_code` 128, `status_message` indiquant que l'exécutable n'a pas été trouvé
- Logs : événement `rejected` ou `executed` avec erreur d'exécution

**Risque couvert** : Exécutable manquant — la config est valide mais l'exécutable a été supprimé ou jamais déployé.

---

## SC-TMO-008 : Exécutable sans permission d'exécution

**Contexte** : Config avec un chemin vers un fichier existant mais sans bit d'exécution.

```toml
[domains.infra.actions.no-exec]
description = "Pas de permission"
level = "read"
execute = "/usr/local/bin/no-exec.sh"
```

(Le fichier existe mais `chmod 644 /usr/local/bin/no-exec.sh`)

**Action** :
```

infra no-exec
.
```

**Attendu** :
- Code de sortie : 128 ou code d'erreur du système
- Réponse JSON : `status_code` indiquant l'erreur d'exécution
- Logs : événement avec raison mentionnant les permissions

**Risque couvert** : Permission manquante — erreur de déploiement courante, doit être signalée proprement.

---

## SC-TMO-009 : Processus enfant qui fork et crée des zombies

**Contexte** : Config standard. Le processus enfant crée des sous-processus.

```toml
[domains.infra.actions.forker]
description = "Crée des sous-processus"
level = "read"
timeout = 5
execute = "/usr/local/bin/forker.sh"
```

**Action** :
```

infra forker
.
```

(Le script lance des sous-processus : `for i in {1..10}; do sleep 60 & done; wait`)

**Attendu** :
- Après le timeout : SIGTERM/SIGKILL envoyé au **process group** (pas seulement au processus principal)
- Tous les sous-processus sont tués
- Code de sortie : 130
- Pas de processus orphelin

**Risque couvert** : Process group kill — le timeout doit tuer tout l'arbre de processus, pas seulement le parent.

---

## SC-TMO-010 : Commande avec timeout = 0

**Contexte** : Config avec un timeout de 0 secondes.

```toml
[domains.infra.actions.instant]
description = "Timeout immédiat"
level = "read"
timeout = 0
execute = "/usr/local/bin/check.sh"
```

**Action** :
```

infra instant
.
```

**Attendu** :
- Comportement défini : soit la commande est rejetée (timeout invalide), soit elle est immédiatement tuée
- Code de sortie : 129 (config invalide) ou 130 (timeout immédiat)
- Logs : événement correspondant

**Risque couvert** : Timeout zéro — cas limite qui ne doit pas causer de comportement indéfini.
