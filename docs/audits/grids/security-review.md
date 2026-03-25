# Grille d'analyse de sécurité — SSH Frontière

**Date de création** : 2026-03-25
**Référence** : ADR 0013 (phase 3), note de recherche 008
**Statut** : Référentiel permanent — applicable à chaque audit de sécurité et PR

---

## Mode d'emploi

Pour chaque axe, l'auditeur :
1. Parcourt chaque point de la colonne « Quoi vérifier »
2. Applique la méthode de la colonne « Comment vérifier »
3. Note le résultat : PASS / FAIL / N/A / À INVESTIGUER
4. Si FAIL, documente la vulnérabilité avec le CWE correspondant et la sévérité
5. Produit un rapport dans `docs/audits/security-audit.md`

**Convention de sévérité** : CRITICAL > HIGH > MEDIUM > LOW > INFO

---

## Axe 1 — Injection et parsing

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 1.1 | Aucune invocation de shell (`/bin/sh`, `/bin/bash`, `Command::new("sh")`) dans tout le code | CWE-78 |
| 1.2 | `std::process::Command` est utilisé exclusivement avec `.arg()` (pas `.args(raw_string)` qui concatène) | CWE-78 |
| 1.3 | Le parseur grammatical (`tokenize_with_quotes` → `resolve_command`) rejette toute entrée non conforme à la grammaire `domaine action [clé=valeur]` | CWE-20 |
| 1.4 | Les limites de taille sont appliquées avant tout parsing (`MAX_LINE_LEN`, `MAX_COMMAND_LEN`, `MAX_TOKEN_LEN`) | CWE-400 |
| 1.5 | Les guillemets (simples et doubles) sont correctement gérés — pas de possibilité d'injection via des guillemets imbriqués ou non fermés | CWE-78 |
| 1.6 | Les arguments `free = true` sont transmis comme valeurs à `.arg()`, pas concaténés dans une chaîne de commande | CWE-88 |
| 1.7 | Les noms de domaine et d'action ne sont jamais utilisés comme chemin de fichier ou dans une construction de commande dynamique | CWE-22 |

### Comment vérifier

```bash
# Recherche d'invocation de shell
grep -rn 'Command::new.*sh\|/bin/sh\|/bin/bash\|Command::new.*cmd' src/ --include="*.rs"

# Recherche de concaténation d'arguments non sûre
grep -rn '\.args(' src/ --include="*.rs"

# Vérifier que .arg() reçoit des valeurs individuelles
grep -rn '\.arg(' src/ --include="*.rs"

# Tester : commande avec pipe, point-virgule, backtick
# $ echo "test" | cat        → doit échouer (syntaxe invalide)
# $ echo "test; rm -rf /"    → le "test; rm -rf /" est un argument valide entre guillemets
# $ echo test; rm -rf /      → doit échouer (syntaxe invalide — le parseur ne gère pas le ;)
```

### Exemples de vulnérabilités classiques

- **Injection de commande via shell** (CWE-78) : `Command::new("sh").arg("-c").arg(user_input)` — l'entrée utilisateur est interprétée par un shell.
- **Argument injection** (CWE-88) : une valeur `--output=/etc/shadow` passée comme argument à un programme qui l'interprète comme une option de fichier de sortie.
- **Path traversal** (CWE-22) : un domaine `../../../etc/passwd` utilisé pour construire un chemin.

### Sévérité par défaut

**CRITICAL** — l'injection de commandes OS est la vulnérabilité la plus grave pour un composant de sécurité. Toute faiblesse dans le parsing compromet le modèle de sécurité entier.

---

## Axe 2 — Authentification

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 2.1 | La comparaison des preuves (proof) utilise `constant_time_eq()`, pas `==` | CWE-208 |
| 2.2 | `constant_time_eq()` utilise `#[inline(never)]` et `black_box` pour empêcher les optimisations LLVM | CWE-208 |
| 2.3 | La comparaison de longueur dans `constant_time_eq()` est acceptable (les preuves sont de longueur fixe — hex SHA-256 = 64 caractères) | CWE-208 |
| 2.4 | Le nonce est généré depuis `/dev/urandom` (pas `/dev/random`, pas `rand()`) | CWE-330 |
| 2.5 | Le nonce est de taille suffisante (≥ 16 octets) | CWE-330 |
| 2.6 | En mode sans nonce (`compute_simple_proof`), la preuve est rejouable — est-ce documenté et accepté ? | CWE-294 |
| 2.7 | Le nombre maximal d'échecs d'authentification est configurable et appliqué (`max_failures`) | CWE-307 |
| 2.8 | Le token d'authentification est correctement validé contre la configuration (lookup exact, pas de regex/glob) | CWE-287 |
| 2.9 | Les secrets en configuration (challenge_nonce, tokens) sont masqués dans les logs (SHA-256 ou `[REDACTED]`) | CWE-532 |

### Comment vérifier

```bash
# Vérifier la comparaison en temps constant
grep -rn 'constant_time_eq\|== .*proof\|proof.* ==' src/ --include="*.rs"

# Vérifier la source d'entropie
grep -rn 'urandom\|random\|rand()' src/ --include="*.rs"

# Vérifier le masquage des secrets dans les logs
grep -rn 'sensitive\|mask\|redact' src/ --include="*.rs"

# Tests : envoyer N+1 preuves invalides, vérifier que la (N+1)ème est rejetée
```

### Exemples de vulnérabilités classiques

- **Timing side channel** (CWE-208) : comparaison `==` sur des secrets, permettant de deviner octet par octet.
- **Replay attack** (CWE-294) : rejouer une preuve valide capturée (mode sans nonce).
- **Brute force** (CWE-307) : pas de limite sur le nombre de tentatives d'authentification.
- **Secret dans les logs** (CWE-532) : le token ou le secret apparaît en clair dans un fichier de log.

### Sévérité par défaut

**HIGH** — une faiblesse d'authentification permet de contourner le contrôle d'accès. CRITICAL si le bypass est trivial (comparaison non constante exploitable en local).

---

## Axe 3 — Contrôle d'accès (RBAC)

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 3.1 | Les niveaux de confiance sont ordonnés correctement (`read < ops < admin`) et la comparaison utilise `PartialOrd` | CWE-285 |
| 3.2 | Les tags sont vérifiés **avant** le niveau de confiance (évite la fuite d'information) | CWE-203 |
| 3.3 | Une action sans tags est accessible à tout niveau suffisant (comportement public documenté) | CWE-285 |
| 3.4 | Le `--level` n'est pas modifiable par le client SSH (forcé par `command=` dans `authorized_keys`) | CWE-269 |
| 3.5 | Un token cross-domaine (tag ne correspondant à aucun domaine) est rejeté | CWE-285 |
| 3.6 | L'absence de `--level` dans les arguments résulte en le niveau le plus restrictif (`read`) | CWE-269 |
| 3.7 | Le niveau de confiance est vérifié pour chaque commande (pas de cache ou d'élévation persistante en session keepalive) | CWE-285 |

### Comment vérifier

```bash
# Vérifier l'ordre des niveaux
grep -rn 'PartialOrd\|Ord.*TrustLevel' src/ --include="*.rs"

# Vérifier l'ordre des vérifications (tags avant niveau)
grep -A5 'check_authorization\|check_tags' src/dispatch.rs

# Vérifier le comportement par défaut de --level
grep -rn 'TrustLevel::Read' src/dispatch.rs

# Tests : envoyer une commande admin avec --level=read → rejet attendu
# Tests : envoyer une commande avec un tag invalide → rejet TagMismatch avant le test de niveau
```

### Exemples de vulnérabilités classiques

- **Privilege escalation** (CWE-269) : un utilisateur `read` peut exécuter une action `admin` via manipulation du `--level`.
- **Information disclosure** (CWE-203) : le message d'erreur distingue « tag invalide » de « niveau insuffisant », révélant l'existence d'actions protégées.
- **Broken access control** (CWE-285) : une action n'est pas vérifiée dans un chemin de code particulier (ex. : mode session keepalive).

### Sévérité par défaut

**HIGH** — un contournement du RBAC permet l'exécution de commandes non autorisées. CRITICAL si escalade vers `admin`.

---

## Axe 4 — Exécution de commandes

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 4.1 | Le chemin de l'exécutable (`exec`) est absolu dans la configuration (pas de résolution via `PATH`) | CWE-426, CWE-427 |
| 4.2 | Le timeout est configuré par action et appliqué via un thread séparé avec `kill` du process group | CWE-400 |
| 4.3 | Le process group est correctement créé (`pre_exec` avec `setsid` ou `setpgid`) pour que le `kill` atteigne tous les enfants | CWE-400 |
| 4.4 | Le code de sortie du processus enfant est correctement propagé (y compris les signaux — `ExitStatusExt`) | CWE-391 |
| 4.5 | Les variables d'environnement du processus enfant sont héritées de sshd — pas de nettoyage explicite | CWE-426 |
| 4.6 | Le body (ADR 0012) est transmis au stdin du processus enfant de manière bornée (`max_body_size`) | CWE-400 |
| 4.7 | Le stdout/stderr du processus enfant est lu de manière bornée ou streamée | CWE-400 |
| 4.8 | En mode session keepalive, chaque commande est validée et exécutée indépendamment | CWE-285 |

### Comment vérifier

```bash
# Vérifier l'exécution directe
grep -rn 'Command::new' src/ --include="*.rs"

# Vérifier les timeouts
grep -rn 'timeout\|kill\|SIGKILL\|SIGTERM' src/ --include="*.rs"

# Vérifier pre_exec / process group
grep -rn 'pre_exec\|setsid\|setpgid\|process_group' src/ --include="*.rs"

# Vérifier les limites de body
grep -rn 'max_body_size\|BodyTooLarge' src/ --include="*.rs"
```

### Exemples de vulnérabilités classiques

- **Untrusted search path** (CWE-426) : un programme exécuté par `PATH` résolution, permettant l'injection d'un faux binaire.
- **Uncontrolled search path element** (CWE-427) : `PATH` contient un répertoire contrôlé par l'attaquant.
- **Resource exhaustion via child process** (CWE-400) : le processus enfant génère du stdout infini, consommant toute la RAM.
- **Zombie process** (CWE-400) : le processus enfant n'est pas correctement attendu (`wait`), accumulant des zombies.

### Sévérité par défaut

**HIGH** — l'exécution de commandes est le coeur fonctionnel. Un défaut ici peut mener à une exécution arbitraire ou un déni de service.

---

## Axe 5 — Gestion des erreurs

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 5.1 | Aucun `unwrap()` dans le code de production (vérifié par clippy `unwrap_used = "deny"`) | CWE-248 |
| 5.2 | Les `expect()` restants sont justifiés par un commentaire `// INVARIANT:` ou situés dans `main()` | CWE-248 |
| 5.3 | Les erreurs sont propagées via `Result<T, E>` avec des types d'erreur structurés (`DispatchError`, `ConfigError`, `ProtocolError`) | CWE-755 |
| 5.4 | Les messages d'erreur renvoyés au client ne contiennent pas d'informations sensibles (chemins internes, secrets, stack traces) | CWE-209 |
| 5.5 | Les codes de sortie sont définis et documentés (ADR 0003) : 128-133 | CWE-391 |
| 5.6 | Un EOF inattendu sur stdin est géré proprement (pas de panic) | CWE-248 |
| 5.7 | Une erreur d'écriture sur stdout (client déconnecté) est gérée proprement | CWE-755 |

### Comment vérifier

```bash
# Rechercher les unwrap restants
grep -rn '\.unwrap()' src/ --include="*.rs" --exclude="*_tests.rs"

# Rechercher les expect sans INVARIANT
grep -rn '\.expect(' src/ --include="*.rs" --exclude="*_tests.rs" | grep -v INVARIANT

# Vérifier les messages d'erreur au client
grep -rn 'status_message\|reason' src/output.rs src/dispatch.rs

# Tests : fermer stdin brutalement pendant la lecture → pas de panic
# Tests : fermer stdout brutalement pendant l'écriture → pas de panic
```

### Exemples de vulnérabilités classiques

- **Uncaught panic** (CWE-248) : un `unwrap()` sur une valeur inattendue termine le processus. En `panic = "abort"`, c'est un crash sans nettoyage.
- **Information exposure via error message** (CWE-209) : le message d'erreur révèle le chemin du fichier de configuration, un numéro de ligne de code, ou la structure interne.
- **Improper error handling** (CWE-755) : une erreur I/O est ignorée silencieusement (`let _ =`), masquant un comportement inattendu.

### Sévérité par défaut

**MEDIUM** — une mauvaise gestion des erreurs peut mener à un crash (DoS) ou à une fuite d'information. HIGH si le crash permet de contourner un contrôle de sécurité.

---

## Axe 6 — Cryptographie

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 6.1 | L'implémentation SHA-256 est testée contre les vecteurs NIST FIPS 180-4 | CWE-327 |
| 6.2 | Le XOR stream cipher (SHA-256 CTR) est documenté comme schéma ad hoc, pas un standard (limitation connue acceptée) | CWE-327 |
| 6.3 | Le compteur XOR (`u8`, max 255) ne wrappera pas pour les tailles de données utilisées (secret + nonce < 8 192 octets) | CWE-327 |
| 6.4 | Le nonce n'est jamais réutilisé (généré aléatoirement pour chaque connexion) | CWE-323 |
| 6.5 | Les secrets ne sont jamais loggés en clair (masquage SHA-256 ou `[REDACTED]`) | CWE-312 |
| 6.6 | Pas de mode ECB, pas de padding oracle, pas de vecteur d'initialisation nul | CWE-329 |
| 6.7 | La base64 est décodée correctement (RFC 4648) — pas de confusion avec d'autres encodages | CWE-838 |

### Comment vérifier

```bash
# Vérifier les tests SHA-256 contre NIST
grep -rn 'NIST\|fips\|test_vector' src/crypto_tests.rs

# Vérifier le compteur XOR
grep -rn 'counter.*wrapping\|counter.*u8' src/crypto.rs

# Vérifier la source du nonce
grep -rn 'generate_nonce\|urandom' src/crypto.rs

# Vérifier le masquage dans les logs
grep -rn 'sensitive\|sha256.*mask\|REDACTED' src/logging.rs
```

### Exemples de vulnérabilités classiques

- **Use of broken crypto** (CWE-327) : MD5 ou SHA-1 pour du hachage de sécurité, DES pour du chiffrement.
- **Reuse of nonce** (CWE-323) : le même nonce utilisé avec la même clé permet de retrouver le XOR des deux messages chiffrés.
- **Cleartext storage of sensitive information** (CWE-312) : le secret stocké en clair dans un fichier lisible par d'autres utilisateurs.
- **Counter wrap** : le compteur revient à 0, réutilisant le même keystream — équivalent à une réutilisation de nonce.

### Sévérité par défaut

**MEDIUM** — le schéma crypto protège l'authentification mais n'est pas la seule couche de défense (SSH lui-même chiffre le transport). HIGH si le contournement est trivial ou si le secret est exposé.

---

## Axe 7 — Déni de service

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 7.1 | Toutes les lectures stdin ont une limite de taille (`MAX_LINE_LEN`, `MAX_COMMAND_LEN`, `max_body_size`) | CWE-770 |
| 7.2 | Le timeout par commande est correctement implémenté et ne peut pas être contourné | CWE-400 |
| 7.3 | Le processus enfant et tous ses descendants sont tués à l'expiration du timeout (kill process group) | CWE-400 |
| 7.4 | En mode session keepalive, le nombre de commandes par session est borné ou le timeout global s'applique | CWE-400 |
| 7.5 | Le programme ne peut pas être forcé à allouer de la mémoire de manière non bornée (pas de `Vec::with_capacity(user_input)`) | CWE-770 |
| 7.6 | Le programme est one-shot (meurt après chaque connexion) — pas d'accumulation d'état | CWE-400 |
| 7.7 | Le nombre de connexions simultanées est limité par sshd (`MaxSessions`, `MaxStartups`), pas par SSH Frontière | CWE-400 |

### Comment vérifier

```bash
# Vérifier les limites de lecture
grep -rn 'MAX_LINE_LEN\|MAX_COMMAND_LEN\|max_body_size\|MAX_TOKEN_LEN' src/ --include="*.rs"

# Vérifier les timeouts
grep -rn 'timeout\|Duration\|sleep\|recv_timeout' src/ --include="*.rs"

# Vérifier les allocations contrôlées par l'entrée
grep -rn 'with_capacity\|Vec::new\|String::new\|Vec::from\|to_vec' src/ --include="*.rs" --exclude="*_tests.rs"

# Tests : envoyer une ligne de 10 Mo → rejet LineTooLong
# Tests : envoyer un body de max_body_size+1 → rejet BodyTooLarge
# Tests : commande qui ne termine jamais → timeout + kill
```

### Exemples de vulnérabilités classiques

- **Billion laughs / XML bomb** (CWE-776) : entrée de petite taille se décompressant en allocation massive. Pas directement applicable (pas de décompression), mais le principe s'applique à un TOML ou JSON profondément imbriqué.
- **Slowloris** (CWE-400) : connexion maintenue ouverte en envoyant des données très lentement, un octet à la fois. En mode session keepalive, chaque connexion bloque un processus sshd.
- **Fork bomb via process enfant** (CWE-400) : le programme autorisé crée des milliers de sous-processus. Mitigation : timeout + kill process group + limites ulimit.

### Sévérité par défaut

**MEDIUM** — un déni de service empêche les utilisateurs légitimes d'accéder au système, mais ne compromet pas la confidentialité ni l'intégrité. HIGH si le DoS est permanent (crash nécessitant un redémarrage).

---

## Axe 8 — Logging et audit

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 8.1 | Chaque tentative (autorisée ou refusée) est loggée avec : timestamp, PID, domaine, action, niveau, tags, résultat | CWE-778 |
| 8.2 | Les arguments sensibles sont masqués dans les logs (SHA-256 quand `sensitive = true` + `mask_sensitive = true`) | CWE-532 |
| 8.3 | Les secrets d'authentification (token, proof, nonce) ne sont jamais loggés en clair | CWE-532 |
| 8.4 | Le format JSON est structuré et parseable (pas de texte libre mélangé) | CWE-117 |
| 8.5 | Les logs ne peuvent pas être injectés (pas de caractère de contrôle, pas de newline dans les valeurs) | CWE-117 |
| 8.6 | L'adresse IP du client SSH est loggée (`SSH_CLIENT`) | CWE-778 |
| 8.7 | L'erreur d'écriture du log (disque plein, permission) ne bloque pas l'exécution de la commande | CWE-755 |
| 8.8 | L'identifiant de session est présent pour corréler les commandes en mode keepalive | CWE-778 |

### Comment vérifier

```bash
# Vérifier le contenu des logs
grep -rn 'LogEntry\|log_entry\|with_domain\|with_action' src/ --include="*.rs"

# Vérifier le masquage
grep -rn 'mask_sensitive\|sensitive.*sha256' src/ --include="*.rs"

# Vérifier la sérialisation JSON (pas de concaténation manuelle)
grep -rn 'serde_json::to_string\|Serialize' src/logging.rs

# Tests : vérifier qu'une commande avec argument sensitive=true est loggée avec le hash, pas la valeur
```

### Exemples de vulnérabilités classiques

- **Log injection** (CWE-117) : un attaquant insère `\n` dans un argument, créant une fausse entrée de log. La sérialisation JSON échappe automatiquement les caractères de contrôle.
- **Sensitive data in logs** (CWE-532) : un mot de passe ou token apparaît en clair dans les logs, accessible à tout utilisateur ayant accès au fichier de log.
- **Insufficient logging** (CWE-778) : les tentatives échouées ne sont pas loggées, rendant la détection d'attaque impossible.

### Sévérité par défaut

**MEDIUM** — une faiblesse de logging ne compromet pas directement la sécurité mais empêche la détection et l'investigation d'incidents. HIGH si des secrets sont loggés en clair.

---

## Axe 9 — Configuration

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 9.1 | Le fichier de configuration est chargé avec des permissions vérifiées (lecture seule pour l'utilisateur SSH) ou la vérification est documentée comme responsabilité de l'opérateur | CWE-732 |
| 9.2 | Les chemins d'exécutables (`exec`) sont absolus — pas de résolution relative ou via `PATH` | CWE-426 |
| 9.3 | Les valeurs par défaut sont sécurisées (fail-closed) : si une option est absente, le comportement est restrictif | CWE-276 |
| 9.4 | Le secret d'authentification est stocké avec le préfixe `b64:` (encodé, pas en clair dans le TOML) | CWE-312 |
| 9.5 | La configuration invalide provoque un arrêt immédiat (fail-fast) avec un code de sortie dédié (129) | CWE-754 |
| 9.6 | Les valeurs numériques (timeout, max_body_size) sont validées pour des bornes raisonnables | CWE-190 |
| 9.7 | Le fichier de configuration n'est pas rechargeable à chaud (pas de SIGHUP) — une modification nécessite un nouveau processus | CWE-367 |

### Comment vérifier

```bash
# Vérifier les chemins absolus dans la config de test
grep -rn 'exec\s*=' tests/fixtures/ docs/ --include="*.toml"

# Vérifier le fail-fast au démarrage
grep -rn 'EXIT_CONFIG_ERROR\|ConfigError' src/ --include="*.rs"

# Vérifier le préfixe b64:
grep -rn 'b64:\|decode_b64_secret' src/ --include="*.rs"

# Tests : config avec exec=relative → doit-il être rejeté ?
# Tests : config avec timeout=0 → comportement ?
# Tests : config avec max_body_size=0 → comportement ?
```

### Exemples de vulnérabilités classiques

- **Insecure default** (CWE-276) : une action est autorisée par défaut si le niveau n'est pas spécifié dans la config.
- **Cleartext secret in config** (CWE-312) : le secret d'authentification est en clair dans un fichier lisible par d'autres utilisateurs.
- **Relative path execution** (CWE-426) : `exec = "backup.sh"` résolu via `PATH`, permettant l'injection d'un binaire malveillant.
- **TOCTOU** (CWE-367) : la config est vérifiée puis rechargée entre la vérification et l'utilisation. N/A si pas de rechargement.

### Sévérité par défaut

**MEDIUM** — une configuration non sécurisée peut mener à une exécution non autorisée ou à une exposition de secrets. HIGH si le défaut est dans la configuration par défaut (pas un choix explicite de l'opérateur).

---

## Axe 10 — Dépendances

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 10.1 | `cargo audit` ne rapporte aucune vulnérabilité connue dans les dépendances de production | CWE-1395 |
| 10.2 | Le nombre de dépendances transitives de production est documenté et jugé acceptable | CWE-1395 |
| 10.3 | Les licences de toutes les dépendances sont compatibles avec EUPL-1.2 (MIT, Apache-2.0, BSD) | — |
| 10.4 | Les mainteneurs des dépendances critiques (`serde`, `toml`) sont identifiés et de confiance | CWE-1395 |
| 10.5 | Le `Cargo.lock` est commité et versionné (builds reproductibles) | CWE-1395 |
| 10.6 | Pas de dépendance avec un `build.rs` suspect (exécution de code arbitraire à la compilation) | CWE-506 |
| 10.7 | Les features activées sont le minimum nécessaire (pas de feature `default` non nécessaire) | CWE-1395 |

### Comment vérifier

```bash
# Audit des vulnérabilités connues
cargo audit

# Compter les dépendances de production
cargo tree --edges normal | grep -v '^\[' | wc -l

# Vérifier les licences
cargo tree --edges normal --format '{p} {l}'

# Lister les build.rs
find . -path '*/build.rs' -not -path './target/*'

# Vérifier les features
grep -A5 '\[dependencies\]' Cargo.toml
```

### Exemples de vulnérabilités classiques

- **Supply chain attack** (CWE-506) : un mainteneur compromis pousse une version malveillante sur crates.io (cf. événements `ua-parser-js` dans l'écosystème npm, `event-stream`).
- **Dependency confusion** : un crate privé est remplacé par un crate public homonyme sur crates.io.
- **Vulnérabilité transitive** (CWE-1395) : une vulnérabilité dans une sous-dépendance (ex. : `hashbrown`, `memchr`) affecte indirectement le projet.
- **Build.rs malveillant** (CWE-506) : un `build.rs` exécute du code arbitraire à la compilation (exfiltration de données, backdoor dans le binaire).

### Sévérité par défaut

**HIGH** — une compromission de la chaîne d'approvisionnement peut injecter du code arbitraire dans le binaire final sans que le code source ne soit modifié. CRITICAL si une vulnérabilité active est détectée par `cargo audit`.

---

## Axe 11 — Build et distribution

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 11.1 | Le binaire release est compilé en statique musl (`x86_64-unknown-linux-musl`) — immunité `LD_PRELOAD` | CWE-426 |
| 11.2 | Le profil release active `lto = true`, `codegen-units = 1`, `strip = true`, `overflow-checks = true` | CWE-693 |
| 11.3 | Le binaire est inférieur à 2 Mo (objectif documenté) | — |
| 11.4 | La CI exécute `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo audit` + tests | CWE-693 |
| 11.5 | Le binaire est déployé avec les permissions minimales (pas de setuid, pas de capabilities excessives) | CWE-250 |
| 11.6 | La cible de build est documentée et reproductible (même toolchain = même binaire) | CWE-1395 |
| 11.7 | `panic = "abort"` est activé en release (pas de déroulement de pile exploitable) | CWE-248 |

### Comment vérifier

```bash
# Vérifier le profil release
grep -A10 '\[profile.release\]' Cargo.toml

# Vérifier les lints
grep -A10 '\[lints' Cargo.toml

# Taille du binaire release
ls -lh target/x86_64-unknown-linux-musl/release/ssh-frontiere 2>/dev/null

# Vérifier que le binaire est statique
file target/x86_64-unknown-linux-musl/release/ssh-frontiere 2>/dev/null
ldd target/x86_64-unknown-linux-musl/release/ssh-frontiere 2>/dev/null
```

### Exemples de vulnérabilités classiques

- **LD_PRELOAD injection** (CWE-426) : un attaquant place une bibliothèque malveillante dans un chemin chargé avant les bibliothèques légitimes. Immunité complète en musl statique.
- **Missing compiler hardening** (CWE-693) : compilation sans protections (pas d'ASLR, pas de stack canary). En Rust, la plupart sont activées par défaut.
- **Setuid binary** (CWE-250) : un binaire setuid avec une vulnérabilité permet une escalade de privilèges root.

### Sévérité par défaut

**MEDIUM** — un défaut de build ou de distribution ne crée pas directement une vulnérabilité mais affaiblit les défenses en profondeur. HIGH si le binaire est déployé avec des permissions excessives.

---

## Axe 12 — Documentation

### Quoi vérifier

| # | Point de contrôle | CWE |
|---|------------------|-----|
| 12.1 | La documentation de déploiement inclut les directives `authorized_keys` avec `restrict` et `command=` | CWE-1059 |
| 12.2 | Les limites de sécurité sont documentées (ce que SSH Frontière protège et ce qu'il ne protège pas) | CWE-1059 |
| 12.3 | Les prérequis de sécurité sont documentés (configuration sshd, permissions fichiers, sudoers) | CWE-1059 |
| 12.4 | Les exemples de configuration ne contiennent pas de secrets réels (tokens, clés) | CWE-312 |
| 12.5 | Le modèle de menace est documenté (qui sont les attaquants, quels sont les vecteurs d'attaque) | CWE-1059 |
| 12.6 | La procédure de rotation des secrets est documentée | CWE-324 |
| 12.7 | La procédure de mise à jour (rollback inclus) est documentée | CWE-1059 |
| 12.8 | Les commentaires de code ne contiennent pas d'informations sensibles (anciens secrets, chemins internes de production) | CWE-615 |

### Comment vérifier

```bash
# Vérifier la documentation de déploiement
ls docs/references/

# Rechercher des secrets dans les exemples et commentaires
grep -rn 'password\|secret\|token.*=.*[a-zA-Z0-9]\{16,\}' docs/ tests/ --include="*.md" --include="*.toml"

# Vérifier la documentation des limites de sécurité
grep -rn 'ne protège pas\|limitation\|hors scope\|out of scope' docs/ --include="*.md"
```

### Exemples de vulnérabilités classiques

- **Secret dans la documentation** (CWE-312) : un token de production apparaît dans un README ou un exemple de configuration commité.
- **Documentation trompeuse** (CWE-1059) : la documentation affirme que SSH Frontière protège contre X alors qu'il ne le fait pas, donnant un faux sentiment de sécurité.
- **Information dans les commentaires** (CWE-615) : un commentaire de code contient un ancien mot de passe ou un chemin d'infrastructure interne.

### Sévérité par défaut

**LOW** — un défaut de documentation ne crée pas directement une vulnérabilité. MEDIUM si la documentation induit en erreur sur les protections offertes, menant à une configuration non sécurisée.

---

## Récapitulatif des sévérités par axe

| Axe | Sévérité par défaut | Justification |
|-----|---------------------|---------------|
| 1. Injection et parsing | **CRITICAL** | Compromet le modèle de sécurité entier |
| 2. Authentification | **HIGH** | Contournement du contrôle d'accès |
| 3. Contrôle d'accès (RBAC) | **HIGH** | Exécution de commandes non autorisées |
| 4. Exécution de commandes | **HIGH** | Exécution arbitraire ou DoS |
| 5. Gestion des erreurs | **MEDIUM** | Crash (DoS) ou fuite d'information |
| 6. Cryptographie | **MEDIUM** | Schéma ad hoc, mais couche SSH en plus |
| 7. Déni de service | **MEDIUM** | Disponibilité, pas intégrité/confidentialité |
| 8. Logging et audit | **MEDIUM** | Détection d'incidents compromise |
| 9. Configuration | **MEDIUM** | Dépend de l'opérateur |
| 10. Dépendances | **HIGH** | Code tiers exécuté dans le processus |
| 11. Build et distribution | **MEDIUM** | Affaiblit la défense en profondeur |
| 12. Documentation | **LOW** | Indirect, mais important pour les opérateurs |
