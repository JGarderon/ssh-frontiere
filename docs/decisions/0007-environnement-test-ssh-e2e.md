# ADR 0007 — Environnement de test SSH end-to-end

**Date** : 2026-03-16
**Statut** : Acceptée
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : ADR 0006 (protocole d'entêtes), ADR 0003 (contrat d'interface JSON), ADR 0002 (secrets et challenge-response), TODO-019 (validation SSH réel), Audit Phase 3
**Phase** : 4 — Validation E2E

---

## Contexte

### Le problème

Tous les tests d'intégration existants (21 scénarios dans `tests/integration.rs` + fixtures de conformance) utilisent un harness `std::process::Command` avec `Stdio::piped()`. Le binaire `ssh-frontiere` est spawné directement comme processus enfant — **aucun `sshd` n'intervient**.

Ce harness valide la logique applicative (parsing, RBAC, exécution, protocole d'entêtes) mais ne couvre **aucun** des comportements suivants :

| Comportement | Couvert par Stdio ? | Risque réel |
|---|---|---|
| Buffering SSH channel vs pipe mémoire | Non | `flush()` insuffisant → client bloqué en attente de bannière |
| Timing des I/O sur réseau (même localhost) | Non | Race condition entre bannière et première écriture client |
| `command=` dans `authorized_keys` | Non | Parsing des arguments `--level`, `--config` par sshd |
| `restrict` dans `authorized_keys` | Non | Interactions avec PTY, forwarding, agent, X11 |
| `PermitUserEnvironment` / `SetEnv` | Non | Variables d'environnement absentes ou inattendues |
| PTY vs non-PTY (ssh -T vs ssh sans -T) | Non | Buffering ligne vs bloc, signaux différents (SIGHUP) |
| Séquences fin de ligne SSH (\r\n vs \n) | Non | Parsing `read_line()` perturbé |
| Fermeture de connexion côté client | Non | SIGPIPE, broken pipe, processus zombie |
| Signaux SSH (SIGHUP à la déconnexion) | Non | Process group non nettoyé |
| Multiplexage (ControlMaster) | Non | État partagé entre connexions |
| Lecture stdin bloquante dans sshd | Non | Timeout session non respecté si `read_line()` bloque indéfiniment |
| `SSH_CLIENT`, `SSH_CONNECTION` env vars | Non | Logging IP source, `ban_command` avec `{ip}` |

L'audit de fin de Phase 3 note explicitement cette lacune, et le TODO-019 la qualifie de priorité haute.

### Pourquoi maintenant

Le protocole d'entêtes (ADR 0006) a changé fondamentalement l'interaction : stdin/stdout sont désormais le canal de protocole. Chaque connexion SSH implique un échange multi-lignes (bannière → entêtes → commande → réponse → [boucle]). Les risques de buffering et timing sont **multipliés** par rapport au mode one-shot des Phases 1-2 où une seule commande était passée via `-c`.

---

## Décision

### 1. Architecture de l'environnement

Deux conteneurs Docker orchestrés par Docker Compose, sur un réseau interne isolé :

```
┌────────────────────────────────┐      ┌────────────────────────────────┐
│         ssh-e2e-server         │      │         ssh-e2e-client         │
│  Debian-Slim                   │      │  Debian-Slim                   │
│                                │      │                                │
│  /usr/local/bin/ssh-frontiere  │ ←──→ │  ssh e2e-user@server ...       │
│  /etc/ssh/sshd_config          │  22  │  /usr/bin/ssh                  │
│  authorized_keys (command=,    │      │  /home/e2e/.ssh/id_ed25519    │
│    restrict)                   │      │                                │
│  /etc/ssh-frontiere/config.toml│      │  Scripts de test (bash/python) │
│  /var/log/ssh-frontiere/       │      │                                │
└────────────────────────────────┘      └────────────────────────────────┘
         réseau interne : ssh-e2e-net (pas d'accès externe)
```

#### Conteneur serveur (`ssh-e2e-server`)

- **Base** : `debian:bookworm-slim`
- **Paquets** : `openssh-server` uniquement
- **Utilisateur** : `e2e-user` (compte de service, shell = `/usr/local/bin/ssh-frontiere`)
- **sshd_config** : configuration restrictive (`PasswordAuthentication no`, `PubkeyAuthentication yes`, `PermitUserEnvironment no`, `AllowUsers e2e-user`, `MaxSessions 10`)
- **authorized_keys** (entrée principale, `--level=read`) : `command="/usr/local/bin/ssh-frontiere --level=read --config=/etc/ssh-frontiere/config.toml",restrict ssh-ed25519 AAAA...`
  - Entrée secondaire (pour AUT-010, `--level=ops`) : `command="/usr/local/bin/ssh-frontiere --level=ops --config=/etc/ssh-frontiere/config.toml",restrict ssh-ed25519 BBBB...`
- **Binaire** : `ssh-frontiere` compilé sur le host (cible `x86_64-unknown-linux-musl`, binaire statique), copié dans le conteneur via volume mount
- **Configuration** : `config.toml` de test avec domaines, actions, et section `[auth.tokens]` pour les scénarios RBAC
- **Logs** : `/var/log/ssh-frontiere/commands.json` monté en volume pour inspection depuis le client

#### Conteneur client (`ssh-e2e-client`)

- **Base** : `debian:bookworm-slim`
- **Paquets** : `openssh-client`
- **Clé SSH** : `ssh-keygen -t ed25519` à la construction, clé publique copiée dans le `authorized_keys` du serveur
- **Scripts de test** : dans `/tests/e2e-ssh/` (montés en volume), exécutables depuis le conteneur
- **known_hosts** : pré-configuré ou `StrictHostKeyChecking=no` (acceptable dans un réseau isolé de test)

#### Compilation du binaire

Le binaire est compilé **sur le host** avant le lancement des conteneurs. Pas de multi-stage build — le binaire statique musl est directement monté en volume :

```bash
make release  # ou cargo build --release --target x86_64-unknown-linux-musl
# Le docker-compose monte ./target/x86_64-unknown-linux-musl/release/ssh-frontiere
```

**Justification** : le multi-stage build nécessiterait l'installation du toolchain Rust dans le conteneur (~1 Go), allongeant le build de plusieurs minutes. Le binaire statique musl est auto-suffisant — il n'a besoin d'aucune dépendance dans le conteneur.

#### Docker Compose

```yaml
# tests/e2e-ssh/docker-compose.yml
services:
  server:
    build: ./server
    volumes:
      - ../../target/x86_64-unknown-linux-musl/release/ssh-frontiere:/usr/local/bin/ssh-frontiere:ro
      - ./config.toml:/etc/ssh-frontiere/config.toml:ro
      - ssh-logs:/var/log/ssh-frontiere
      - ssh-keys:/ssh-keys:ro
    networks:
      - ssh-e2e-net

  client:
    build: ./client
    volumes:
      - ./scenarios:/tests:ro
      - ssh-logs:/var/log/ssh-frontiere:ro
      - ssh-keys:/ssh-keys
    networks:
      - ssh-e2e-net
    depends_on:
      server:
        condition: service_healthy

volumes:
  ssh-logs:
  ssh-keys:

networks:
  ssh-e2e-net:
    internal: true
```

Le serveur expose un healthcheck (`ssh-keyscan localhost`) pour que le client ne démarre que quand sshd est prêt.

### 2. Pyramide de tests et insertion des tests E2E

```
                    ╱╲
                   ╱  ╲         E2E SSH (cette ADR)
                  ╱ E2E╲        56 scénarios, Docker, ~10-30s (images buildées)
                 ╱──────╲       Valide : transport SSH, buffering, timing,
                ╱        ╲      authorized_keys, signaux, réseau
               ╱──────────╲
              ╱ Intégration ╲   tests/integration.rs + conformance.rs
             ╱   (existant)  ╲  ~25 scénarios, Stdio::piped(), rapide (~5s)
            ╱                 ╲ Valide : logique protocole, RBAC, exécution
           ╱───────────────────╲
          ╱     Unitaires       ╲ *_tests.rs (protocol, crypto, dispatch, etc.)
         ╱     (existant)        ╲ ~100+ tests, < 1s
        ╱  Valide : parsing,      ╲
       ╱  crypto, config, types    ╲
      ╱─────────────────────────────╲
```

#### Quand lancer les tests E2E

| Contexte | Lancement E2E ? | Justification |
|---|---|---|
| Développement local | Oui (`make e2e`) | Compatible boucle TDD (~10-30s : conteneurs Debian-Slim démarrent en < 1s, sshd est léger, chaque connexion SSH ~ quelques ms) [correction Julien BO] |
| Pré-commit | Oui | Compatible pré-commit (~10-30s une fois les images buildées) [correction Julien BO] |
| CI (push sur branche) | Oui | Valide les changements avant merge |
| CI (merge sur main) | Oui | Gate obligatoire, pas de merge sans E2E vert |
| Modification `authorized_keys` / sshd_config | Obligatoire | Ce sont exactement les cas que les E2E couvrent |

#### Relation avec les tests existants

Les tests E2E **ne remplacent pas** les tests d'intégration Stdio. Les deux niveaux coexistent :

- **Tests Stdio** : boucle de feedback rapide, couvrent la logique applicative, s'exécutent sans Docker
- **Tests E2E SSH** : validation de bout en bout, couvrent le transport, s'exécutent avec Docker

Un scénario qui passe en Stdio mais échoue en E2E SSH révèle un problème de **transport** (buffering, timing, signaux). L'inverse (E2E passe, Stdio échoue) ne devrait pas arriver — si c'est le cas, c'est un bug dans le harness Stdio.

### 3. Configuration de test E2E

Le fichier `config.toml` de test E2E est dédié (distinct de `tests/fixtures/test-config.toml`) et conçu pour couvrir tous les scénarios :

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 10
default_level = "read"
mask_sensitive = true
max_stdout_chars = 65536
max_stderr_chars = 16384
max_output_chars = 131072
timeout_session = 30       # Court pour tester le timeout en E2E
max_auth_failures = 3
log_comments = true
ban_command = ""           # Pas de ban réel en test

[auth.tokens.runner-e2e]
secret = "b64:c2VjcmV0LXJ1bm5lci1lMmU="
level = "ops"

[auth.tokens.admin-e2e]
secret = "b64:c2VjcmV0LWFkbWluLWUyZQ=="
level = "admin"

# Philosophie auth (Julien BO) : c'est le `level` de chaque action qui
# détermine si un +auth est nécessaire, pas un « niveau de connexion ».
# Avec --level=read (entry principale), echo/slow/fail/say/big-output sont
# accessibles sans auth, greet exige +auth (level=ops), admin.status exige
# +auth admin.

[domains.test]
description = "Domaine de test E2E"

[domains.test.actions.echo]
description = "Echo simple"
level = "read"
timeout = 5
execute = "/bin/echo {domain}"
args = []

[domains.test.actions.greet]
description = "Salutation"
level = "ops"
timeout = 5
execute = "/bin/echo hello {name}"
args = [{ name = "name", arg_type = "enum", values = ["world", "e2e"] }]

[domains.test.actions.slow]
description = "Commande lente (timeout)"
level = "read"
timeout = 1
execute = "/bin/sleep 10"
args = []

[domains.test.actions.fail]
description = "Commande en echec"
level = "read"
timeout = 5
execute = "/bin/false"
args = []

[domains.test.actions.say]
description = "Echo avec argument texte libre"
level = "read"
timeout = 5
execute = "/bin/echo {message}"
args = [{ name = "message", arg_type = "string" }]

[domains.test.actions.big-output]
description = "Sortie volumineuse"
level = "read"
timeout = 5
execute = "/usr/bin/seq 1 10000"
args = []

[domains.admin]
description = "Actions admin"

[domains.admin.actions.status]
description = "Statut systeme"
level = "admin"
timeout = 5
execute = "/bin/echo admin-ok"
args = []
```

### 4. Scénarios de test — catalogue exhaustif

Chaque scénario sera implémenté en TDD (RED : script de test écrit d'abord, échoue → GREEN : fix si nécessaire → RESOLUTION : nettoyage). Les scripts de test sont écrits en bash, exécutés depuis le conteneur client.

#### Convention de nommage

```
E2E-SSH-PPP-NNN : catégorie PPP, numéro NNN
```

Les scripts retournent `0` si le scénario passe, `1` sinon, avec un message explicatif sur stderr.

---

#### 4.1 Protocole de base (E2E-SSH-PRO-*)

Ces scénarios valident que le protocole d'entêtes (ADR 0006) fonctionne correctement à travers un vrai canal SSH.

| ID | Scénario | Description | Vérification |
|---|---|---|---|
| PRO-001 | Bannière reçue à la connexion | Connexion SSH, lecture immédiate de stdout | La première ligne contient `# ssh-frontiere`, suivi de `+ capabilities` avec `session, help` |
| PRO-002 | Challenge présent si auth configurée | Connexion vers serveur avec `[auth.tokens]` | La bannière contient `+ challenge nonce=` suivi de 32 caractères hexadécimaux |
| PRO-003 | Challenge absent si auth non configurée | Connexion vers serveur sans `[auth]` | La bannière ne contient aucune ligne `+ challenge` et capabilities ne contient pas `rbac` |
| PRO-004 | Entêtes client acceptés | Envoi de `# commentaire test` + ligne vide + `$ test echo` | Réponse `>` JSON avec `status_code=0` |
| PRO-005 | Ligne vide sépare entêtes et commande | Envoi de `$ test echo` **sans** ligne vide d'abord | Le serveur interprète `$` comme un entête invalide ou attend toujours la ligne vide (comportement à vérifier — le `$` en phase d'entêtes est une erreur de protocole selon ADR 0006 §10) |
| PRO-006 | Commande exécutée, réponse JSON 4 champs | `$ test echo` après entêtes corrects | Réponse `> {"status_code": 0, "status_message": "executed", "stdout": "...", "stderr": ...}` — les 4 champs présents |
| PRO-007 | Connexion fermée après réponse (one-shot) | Envoi d'une commande sans `+session keepalive` | Après la réponse `>`, la connexion SSH se ferme (code de retour SSH = 0) |
| PRO-008 | Commentaires serveur dans la bannière | Vérification de la ligne `# type "$ help"...` | Présente dans la bannière, préfixée par `#` |
| PRO-009 | Préfixe `>` sur la réponse JSON | Vérification du format exact | La ligne de réponse commence par `> ` suivi de JSON valide |
| PRO-010 | Flush de la bannière | Le client reçoit la bannière **avant** d'envoyer quoi que ce soit | Mesure : la bannière est lisible dans le premier read après connexion, sans écriture préalable du client |

---

#### 4.2 Authentification (E2E-SSH-AUT-*)

Ces scénarios valident le challenge-response (ADR 0006 §3) et le RBAC (ADR 0004, 0006 §4-5) sur un vrai canal SSH.

> **Recadrage fondamental (Julien BO)** : l'authentification n'est pas une obligation mais une configuration possible. Certaines actions sont accessibles sans `+auth`, d'autres exigent un `+auth` — c'est la **configuration TOML** (`[domains.<id>.actions.<id>].level`) qui le détermine, pas un « niveau implicite » de la connexion. Il n'y a pas de modèle « read = public / ops = protégé » — il y a une configuration par action.
>
> Réf. : Exercice d'alignement 002, concept 3.2 (Julien BO) : « L'authentification est posée par le RBAC, mais le RBAC n'impose pas d'avoir un mode "anonyme". »

| ID | Scénario | Description | Vérification |
|---|---|---|---|
| AUT-001 | Action dont le TOML n'exige pas d'auth | `test.echo` configuré `level = "read"` dans le TOML, connexion `--level=read`, pas de `+auth`, `$ test echo` | `status_code=0` — le TOML autorise cette action au niveau de base de la connexion |
| AUT-002 | Action dont le TOML exige un niveau supérieur, sans +auth | `test.greet` configuré `level = "ops"` dans le TOML, connexion `--level=read`, pas de `+auth`, `$ test greet world` | `status_code=131` — le TOML exige `ops`, la connexion n'a que `read` |
| AUT-003 | Action exigeant auth + +auth valide | `test.greet` (level=ops), `+auth token=runner-e2e proof=<valid>` (token level=ops), `$ test greet world` | `status_code=0` — le token élève le niveau effectif à `ops`, conforme au `level` configuré dans le TOML |
| AUT-004 | +auth avec proof invalide | `+auth token=runner-e2e proof=invalide` | Commentaire `# auth failed (1/3)`, le niveau effectif reste celui de la connexion de base |
| AUT-005 | 3 +auth invalides : déconnexion | 3 envois de `+auth` avec proof incorrect | Commentaire `# auth failed (3/3)`, connexion fermée, code de sortie 132 |
| AUT-006 | Nonce différent à chaque connexion | 2 connexions SSH successives | Les nonces dans `+ challenge nonce=` sont différents |
| AUT-007 | Proof valide avec nonce de la connexion | Calcul du proof `SHA-256(XOR_encrypt(secret \|\| nonce, secret))` | Authentification acceptée, commentaire `# auth ok` |
| AUT-008 | Proof d'une connexion précédente rejeté (anti-replay) | Capturer le nonce de la connexion 1, envoyer le proof de la connexion 1 lors de la connexion 2 | Authentification rejetée — le nonce est unique par connexion |
| AUT-009 | Token inconnu rejeté | `+auth token=inexistant proof=abc` | Compté comme échec d'auth |
| AUT-010 | Niveau effectif = max(base, token) | Connexion `--level=ops` (2e entry authorized_keys), `+auth` avec token `read` | Le niveau effectif reste `ops` (max des deux), conforme à ADR 0006 §4 |

---

#### 4.3 Mode session (E2E-SSH-SES-*)

Ces scénarios valident le mode multi-commandes (ADR 0006 §6) sur un vrai canal SSH, où le timing et le buffering sont critiques.

| ID | Scénario | Description | Vérification |
|---|---|---|---|
| SES-001 | +session keepalive : connexion reste ouverte | `+session keepalive`, ligne vide, `$ test echo` | Après la réponse `>`, la connexion ne se ferme pas — le client peut envoyer une autre commande |
| SES-002 | Plusieurs commandes successives | `+session keepalive`, 3 commandes `$ test echo` puis `$ exit` | 3 réponses `>` distinctes reçues, chacune avec un JSON valide |
| SES-003 | Changement de +auth en session | `+auth token=runner-e2e proof=<valid>`, `$ test greet world`, `+auth token=admin-e2e proof=<valid>`, `$ admin status` | Première commande OK au niveau ops, deuxième OK au niveau admin |
| SES-004 | $ exit ferme la session | `+session keepalive`, `$ test echo`, `$ exit` | Réponse à `exit` avec `status_code=0`, puis connexion fermée proprement |
| SES-005 | EOF ferme la session | `+session keepalive`, `$ test echo`, puis fermeture de stdin | La connexion SSH se termine proprement (pas de crash, pas de zombie) |
| SES-006 | Timeout de session | `+session keepalive`, `$ test echo`, puis inactivité > `timeout_session` (30s en config E2E) | Le serveur ferme la connexion avec un commentaire `# session timeout` |
| SES-007 | Réponse complète entre chaque commande | `+session keepalive`, `$ test echo`, attente de `>`, puis `$ test echo` | La deuxième commande n'est envoyée qu'après réception complète de la première réponse — pas d'entrelacement |
| SES-008 | Commentaires # en cours de session | `+session keepalive`, `# note de debug`, `$ test echo` | Le commentaire est ignoré par le protocole, la commande s'exécute normalement |

---

#### 4.4 Sécurité (E2E-SSH-SEC-*)

Ces scénarios valident les mécanismes de sécurité dans un contexte SSH réel, avec les restrictions de `authorized_keys`.

> **Principe fondamental (Julien BO)** : ssh-frontiere **n'est pas un shell**. La sécurité repose sur le **parseur grammatical**, pas sur du filtrage de caractères (liste noire). La grammaire attendue est `domaine action [args]` — tout ce qui ne respecte pas cette grammaire est rejeté par le parseur. Les caractères spéciaux (`|`, `;`, `&`, `$`, etc.) entre guillemets sont du contenu d'argument, pas de la syntaxe shell. Il n'y a pas de « caractères interdits » — il y a une grammaire, et ce qui ne respecte pas la grammaire est rejeté.
>
> Citation Julien (BO) : « `$ echo "|"` est valide. Il faut donc simplement laisser le parseur faire son travail. `$ echo "ok" | cmd` n'est pas interdit mais simplement n'est pas valide dans la grammaire. »

| ID | Scénario | Description | Vérification |
|---|---|---|---|
| SEC-001 | restrict bloque le port-forwarding | `ssh -L 8080:localhost:80 e2e-user@server` | Connexion SSH refuse le forwarding local (`administratively prohibited` ou ignoré) |
| SEC-002 | restrict bloque le X11 forwarding | `ssh -X e2e-user@server` | Pas de forwarding X11 (variable `DISPLAY` absente) |
| SEC-003 | restrict bloque l'agent forwarding | `ssh -A e2e-user@server` | Forwarding agent bloqué |
| SEC-004 | restrict bloque le PTY | `ssh e2e-user@server` (sans -T) | Pas de PTY alloué (`no-pty` dans restrict), le protocole fonctionne quand même |
| SEC-005 | Rejet grammatical : tokens excédentaires avec pipe | `$ test echo \| /bin/cat /etc/passwd` | `status_code=128` — le parseur attend 0 arguments pour `echo`, reçoit 3 tokens excédentaires. Rejet par la **grammaire** (nombre d'arguments incorrect), pas par filtrage de caractère |
| SEC-006 | Rejet grammatical : tokens excédentaires avec point-virgule | `$ test echo ; /bin/id` | `status_code=128` — le `;` est un token comme un autre pour le parseur, pas un opérateur. Rejet par nombre d'arguments incorrect |
| SEC-007 | Rejet grammatical : tokens excédentaires avec esperluette | `$ test echo & /bin/id` | `status_code=128` — même logique : tokens excédentaires selon la grammaire |
| SEC-008 | Cas positif : caractères spéciaux entre guillemets | `$ test say "hello\|world;test&foo$bar"` | `status_code=0` — les caractères `\|`, `;`, `&`, `$` entre guillemets sont du **contenu** d'argument (type `string`), pas de la syntaxe. Le parseur grammatical les accepte comme valeur de l'argument `message` |
| SEC-009 | Cas positif : variable shell non interprétée | `$ test say "$HOME"` | `status_code=0` — le parseur traite `$HOME` comme une chaîne littérale. ssh-frontiere n'est pas un shell, pas d'expansion de variable |
| SEC-010 | Commande inconnue rejetée | `$ unknown action` | `status_code=128`, `stdout=null` |
| SEC-011 | Arguments excédentaires rejetés | `$ test echo arg1 arg2 arg3` | `status_code=128` (trop d'arguments selon la grammaire) |
| SEC-012 | Ligne sans préfixe rejetée | Envoi de `forgejo healthcheck` (sans `$`) en phase commande | Erreur de protocole (code 132) |
| SEC-013 | command= force le binaire | Tentative `ssh e2e-user@server /bin/bash` | Le `command=` dans authorized_keys force ssh-frontiere, pas /bin/bash — le binaire reçoit `-c /bin/bash` qu'il ignore (ADR 0006) |
| SEC-014 | Pas d'accès shell même avec PTY demandé | `ssh -tt e2e-user@server` | Pas de shell interactif — le protocole s'exécute (restrict + command= empêchent le shell) |

---

#### 4.5 Robustesse (E2E-SSH-ROB-*)

Ces scénarios valident le comportement du serveur dans des conditions dégradées ou anormales, spécifiques au transport SSH.

| ID | Scénario | Description | Vérification |
|---|---|---|---|
| ROB-001 | Déconnexion brutale du client | Le client se connecte, envoie les entêtes, puis `kill -9` du processus SSH | Le serveur (ssh-frontiere) ne crash pas, pas de processus zombie. Vérifié par `ps aux` sur le serveur après 2s |
| ROB-002 | Commande très longue (limite 4096) | Envoi d'une ligne `$ test echo ` + 4080 caractères | Erreur de protocole, code 132, message `line too long` |
| ROB-003 | Réponse volumineuse | `$ test big-output` (seq 1 10000) | Réponse reçue complète, stdout tronqué selon `max_stdout_chars`, JSON valide |
| ROB-004 | Connexions simultanées | 5 connexions SSH en parallèle, chacune exécutant `$ test echo` | Les 5 obtiennent une réponse correcte, pas de corruption inter-connexions |
| ROB-005 | Encoding UTF-8 dans les commentaires | `# Commentaire avec accents : éèêë, guillemets « » et emoji 🔒` | Le commentaire est transmis sans corruption via SSH, loggé correctement si `log_comments=true` |
| ROB-006 | Entête très long (< 4096) | Envoi de `# ` suivi de 4090 caractères | Accepté sans erreur (sous la limite de 4096) |
| ROB-007 | Ligne vide dans le flux en session | En mode session, envoi d'une ligne vide entre deux commandes | La ligne vide n'est pas interprétée comme fin d'entêtes en phase commande — comportement à documenter |
| ROB-008 | SIGPIPE propagé correctement | Client ferme stdout avant que le serveur ait fini d'écrire | Le serveur détecte l'erreur d'écriture et s'arrête proprement |

---

#### 4.6 Logging (E2E-SSH-LOG-*)

Ces scénarios valident que le logging JSON fonctionne correctement dans un contexte SSH réel, avec les variables d'environnement SSH positionnées par sshd.

| ID | Scénario | Description | Vérification |
|---|---|---|---|
| LOG-001 | Commande exécutée → entrée JSON dans le log | `$ test echo` | Fichier `/var/log/ssh-frontiere/commands.json` contient une entrée JSON avec `event=executed`, `domain=test`, `action=echo` |
| LOG-002 | SSH_CLIENT capturé dans le log | `$ test echo` | L'entrée JSON contient `ssh_client` non null, avec l'IP du conteneur client |
| LOG-003 | Commentaires loggés si configuré | `# commentaire de test` puis `$ test echo` | L'entrée de log contient le commentaire (car `log_comments=true` dans la config E2E) |
| LOG-004 | Échec d'auth loggé | `+auth token=runner-e2e proof=invalide` | Entrée JSON avec `event` indiquant l'échec d'auth, `ssh_client` présent |
| LOG-005 | Timestamp cohérent | `$ test echo` | L'entrée JSON a un `timestamp` au format ISO 8601 (`YYYY-MM-DDTHH:MM:SSZ`), et la date/heure est cohérente (pas de dérive > 60s) |
| LOG-006 | Arguments sensibles masqués | Commande avec argument `sensitive=true` | L'argument est masqué par SHA-256 dans le log (pas la valeur en clair) |

---

### 5. Implémentation des scripts de test

Chaque scénario est un script bash autonome dans `tests/e2e-ssh/scenarios/` :

```bash
#!/bin/bash
# E2E-SSH-PRO-001 : Bannière reçue à la connexion
set -euo pipefail

RESULT=$(ssh -T -o BatchMode=yes e2e-user@server <<'PROTOCOL'
PROTOCOL
)

# La bannière est envoyée même sans rien écrire (on envoie EOF immédiat)
echo "$RESULT" | grep -q "^# ssh-frontiere" || {
    echo "FAIL: bannière absente" >&2; exit 1
}
echo "$RESULT" | grep -q "^+ capabilities" || {
    echo "FAIL: capabilities absentes" >&2; exit 1
}
echo "PASS: PRO-001"
```

Pour les scénarios nécessitant un calcul de proof (AUT-003, AUT-007, etc.), un script utilitaire `compute-proof.sh` (ou un petit binaire dédié) sera fourni, capable de :
1. Extraire le nonce de la bannière
2. Calculer `SHA-256(XOR_encrypt(secret || nonce, secret))`
3. Émettre le proof en hexadécimal

Un runner global `run-all.sh` exécute tous les scénarios et produit un rapport :

```
E2E-SSH-PRO-001  PASS  Bannière reçue à la connexion
E2E-SSH-PRO-002  PASS  Challenge présent si auth configurée
E2E-SSH-AUT-003  FAIL  Action exigeant auth + +auth valide
...
56/56 passed, 0 failed
```

### 6. Intégration Make et CI

```makefile
# Makefile (additions)
e2e-build:
	cargo build --release --target x86_64-unknown-linux-musl

e2e-up: e2e-build
	cd tests/e2e-ssh && docker compose up -d --build --wait

e2e-test: e2e-up
	cd tests/e2e-ssh && docker compose exec client /tests/run-all.sh

e2e-down:
	cd tests/e2e-ssh && docker compose down -v

e2e: e2e-test e2e-down
```

En CI :
```yaml
e2e-ssh:
  needs: [build-musl]
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Download musl binary
      uses: actions/download-artifact@v4
    - name: Run E2E SSH tests
      run: make e2e
```

---

## Relecture agressive

### PM tatillon

**Q : Pourquoi Docker et pas un sshd local ?**
R : L'isolation Docker garantit la reproductibilité. Un sshd local dépend de la configuration machine (ports, utilisateurs, clés existantes). Docker Compose donne un environnement identique sur chaque machine de dev et en CI. Pas de « ça marche sur ma machine ».

**Q : 56 scénarios E2E, c'est pas trop ? Le temps de CI va exploser.**
R : Les scénarios sont indépendants et légers (chacun = 1 connexion SSH). Les conteneurs Debian-Slim démarrent en moins d'une seconde, sshd est léger, une connexion SSH prend quelques millisecondes. Une fois les images buildées (rarement), `make e2e` tourne en **10-30 secondes** pour 56 scénarios [estimation corrigée par Julien BO]. Le build musl (déjà en CI) est le vrai goulot — pas les tests eux-mêmes.

**Q : Pourquoi des scripts bash et pas des tests Rust ?**
R : Les tests E2E SSH doivent se comporter comme un **vrai client SSH**. Un test Rust avec `std::process::Command("ssh", ...)` ajouterait une couche d'abstraction inutile par rapport à un script bash qui fait exactement ce que ferait un opérateur. De plus, bash est le langage naturel des scripts de test SSH (heredocs, pipes, signaux). Les scénarios d'injection et de timing sont plus lisibles en bash.

**Q : Le compute-proof.sh, c'est pas fragile ?**
R : C'est le point le plus délicat. Deux options : (1) un script utilisant `openssl dgst` et des outils hexadécimaux — fragile et difficile à maintenir pour le XOR cipher maison, (2) un petit binaire Rust dédié (`ssh-frontiere-proof`) compilé en même temps que le binaire principal — fiable car il réutilise les mêmes fonctions crypto. L'option 2 est recommandée.

**Q : Comment tester PRO-003 (pas d'auth) si le config.toml contient [auth.tokens] ?**
R : Deux authorized_keys entries, deux configs. L'entry par défaut utilise la config avec auth. Une deuxième entry (clé différente) utilise une config **sans** `[auth]`. Le conteneur client a les deux clés.

**Q : Comment tester le timeout session (SES-006) sans attendre 30s ?**
R : Le `timeout_session = 30` dans la config E2E est déjà court. On pourrait le réduire à 5s pour une config spécifique aux tests de timeout, avec une deuxième config et une deuxième entry authorized_keys.

### Tech Lead hargneux

**Q : Le volume mount du binaire, c'est un vecteur d'attaque en prod ?**
R : On est dans un environnement de **test**. En prod, le binaire est déployé via le pipeline de release, pas via Docker. L'ADR ne concerne que l'environnement de test.

**Q : `StrictHostKeyChecking=no` c'est une hérésie.**
R : Acceptable dans un réseau Docker `internal: true` sans accès externe. Alternative : copier la host key du serveur dans le `known_hosts` du client au build. La deuxième option est meilleure, appliquons-la.

**Q : Les tests E2E bash sont non-typés, non-structurés, et vont devenir un cauchemar de maintenance.**
R : Convention stricte : un fichier par scénario, format standard (PASS/FAIL sur stderr, code de sortie), runner qui les agrège. Pas de logique métier dans les scripts — juste connexion SSH + assertions grep. Si ça devient trop complexe, c'est un signal que le scénario est mal découpé.

**Q : ROB-001 (kill -9) — comment vérifier qu'il n'y a pas de zombie ?**
R : `docker exec server ps aux | grep defunct` après un délai de 2s. Si sshd moissonne correctement ses enfants (ce qu'il fait par défaut), il ne devrait pas y avoir de zombie. Si ssh-frontiere fork mal (ce qu'il ne fait pas — il utilise `Command` qui attend le child), pas de zombie non plus.

**Q : ROB-004 (5 connexions parallèles) — ça teste quoi que les tests unitaires ne testent pas ?**
R : Chaque connexion SSH = un nouveau processus `ssh-frontiere`. Les processus ne partagent rien (pas de state global, pas de fichier lockable). Le vrai risque : contention sur le fichier de log (écriture concurrente). Et aussi : vérifier que sshd gère bien le `MaxSessions` configuré.

**Q : SEC-013 (command= force le binaire) — redondant avec la doc SSH ?**
R : Non. La doc SSH dit que `command=` remplace la commande. On vérifie que **notre** déploiement (authorized_keys exact, format command=) fait bien ce qu'on attend. C'est un test de configuration, pas un test d'OpenSSH.

**Q : Comment le binaire ssh-frontiere-proof ne devient pas une dette technique ?**
R : Il est dans `src/bin/` ou `examples/`, réutilise `crypto::compute_proof()` directement. Zéro code dupliqué. Si `compute_proof()` change, le binaire change avec.

**Q : SEC-008/009 (cas positifs avec caractères spéciaux entre guillemets) vont échouer avec le code actuel — `FORBIDDEN_CHARS` dans `dispatch.rs` rejette `|`, `;`, `&`, `$` même entre guillemets.**
R : Exact. Le `FORBIDDEN_CHARS` est un vestige de la Phase 1 (modèle sanitization brute). Avec le protocole d'entêtes, il doit être supprimé au profit du parseur grammatical (recadrage Julien BO). C'est un prérequis d'implémentation : supprimer la vérification `FORBIDDEN_CHARS` dans `parse_command()` et laisser `tokenize_with_quotes()` + `resolve_command()` faire le travail de validation. Le parseur grammatical est intrinsèquement plus sûr qu'une liste noire.

### Vérification de cohérence avec les ADR existantes

| ADR | Point vérifié | Cohérent ? |
|---|---|---|
| 0001 (TOML) | La config E2E utilise le même format TOML | Oui — structure identique avec les champs Phase 3 |
| 0002 (secrets) | Challenge-response testé en E2E (AUT-003, 007, 008) | Oui — vérifie `empreinte(chiffré(secret + T))` bout en bout |
| 0003 (JSON) | Réponse 4 champs vérifiée (PRO-006) | Oui — format identique, codes de sortie conformes |
| 0004 (résolution) | Parsing domaine/action vérifié via SSH réel | Oui — les tests PRO/SEC utilisent le même format `$ domaine action` |
| 0005 (SHA-256) | SHA-256 maison utilisé dans les proofs E2E | Oui — le binaire `ssh-frontiere-proof` utilise la même implémentation |
| 0006 (protocole) | Tous les aspects du protocole couverts | Oui — bannière, entêtes, challenge, session, commentaires, erreurs |

### Vérification de cohérence avec l'exercice d'alignement 002

| Principe (alignement 002) | Scénarios E2E |
|---|---|
| « Un seul protocole, pas deux problèmes » | AUT-* et SES-* testent auth et session sur le même canal |
| « Lisibilité triple » | PRO-001 à PRO-009 vérifient les préfixes lisibles |
| « Mode session opt-in, pas paradigme » | SES-001 vérifie que sans `+session`, la connexion ferme |
| « L'auth affine, elle ne fonde pas » | AUT-001/002 vérifient que c'est le TOML qui détermine l'accessibilité, AUT-003 l'élévation par token |
| « +auth est un préambule, pas un état » | SES-003 vérifie le changement d'auth en session |
| « Défense en profondeur — 3 tentatives + ban » | AUT-004/005 vérifient le compteur et la déconnexion |
| « Bannière obligatoire » | PRO-001/010 vérifient la bannière immédiate |
| « Ligne vide = séparateur » | PRO-004/005 vérifient la séparation entêtes/commande |
| « Pas de rétrocompatibilité » | SEC-012 vérifie le rejet des lignes sans préfixe |

---

## Conséquences

### Positives

- **Confiance** : première validation du protocole sur un vrai canal SSH — réduit les risques identifiés dans TODO-019
- **Reproductibilité** : Docker Compose garantit un environnement identique partout
- **Régression** : les scénarios E2E servent de filet de sécurité pour les évolutions futures du protocole
- **Documentation vivante** : les scripts de test documentent le comportement attendu en conditions réelles

### Négatives

- **Complexité infrastructure** : Docker Compose + Dockerfiles + scripts + runner + binaire utilitaire
- **Temps de CI** : ~30-60s supplémentaires une fois les images Docker buildées (les images sont rarement rebuildées) [corrigé — Julien BO]
- **Maintenance** : 56 scripts bash à maintenir en parallèle des tests Rust
- **Dépendance Docker** : les développeurs sans Docker ne peuvent pas lancer les E2E localement

### Risques

- Le binaire `ssh-frontiere-proof` pourrait diverger de l'implémentation serveur si les tests ne sont pas lancés régulièrement
- Les scripts bash sont fragiles face aux changements de format de sortie (parsing grep)
- Le timeout de 30s pour SES-006 ralentit la suite de tests — envisager un mécanisme de timeout adaptatif en CI

---

## Attribution

- **Julien (BO)** : identification du besoin (TODO-019), validation de l'approche Docker E2E. **Corrections fondamentales de relecture** :
  1. *Timing E2E corrigé* : les conteneurs Debian-Slim démarrent en < 1s, sshd est léger, une connexion SSH ~ quelques ms → `make e2e` en 10-30s pour 56 scénarios, compatible développement local et pré-commit (l'estimation initiale de 2-5 min était surestimée)
  2. *Philosophie d'authentification recadrée* : « L'authentification n'est pas une obligation mais une configuration possible. Certaines actions sont accessibles sans `+auth`, d'autres exigent un `+auth` — c'est la CONFIGURATION TOML qui le détermine, pas un niveau implicite de la connexion. » (réf. alignement 002, concept 3.2). Scénarios AUT réécrits avec cette logique.
  3. *Parseur grammatical vs liste noire* : « ssh-frontiere n'est pas un shell. La sécurité repose sur le parseur grammatical, pas sur du filtrage de caractères. `$ echo "|"` est valide. `$ echo "ok" | cmd` n'est pas interdit mais simplement n'est pas valide dans la grammaire. » Scénarios SEC réécrits avec des cas positifs (caractères spéciaux entre guillemets) et des rejets grammaticaux (pas de « caractères interdits »).
- **Claude (PM/Tech Lead)** : architecture conteneurs, catalogue de scénarios initial, relecture agressive, intégration CI
- **Agents Claude Code** : implémentation
