# Audit - Code Review

Pour **chaque crate** du workspace, audite le code selon les axes suivants, dans cet ordre de priorité.

---

## 0. PROCÉDURE D'EXÉCUTION (agent autonome)

### Étape 0.1 — Découverte du workspace

```bash
# Lister les crates du workspace
cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name'

# Pour chaque crate, identifier le type (bin ou lib)
# bin : contient src/main.rs ou [[bin]] dans Cargo.toml
# lib : contient src/lib.rs ou [lib] dans Cargo.toml
grep -l "fn main" */src/main.rs src/main.rs 2>/dev/null  # binaires
```

### Étape 0.2 — Ordre de traitement

Traiter les crates dans cet ordre :
1. **Crates leaf** (sans dépendance interne au workspace) — ex : `proxy-core`
2. **Crates intermédiaires** — par ordre topologique (`cargo metadata` → `resolve.nodes`)
3. **Crate racine** (binaire principal) — en dernier

Au sein d'un crate, traiter les fichiers dans cet ordre :
1. `lib.rs` / `main.rs` (surface d'API publique)
2. Fichiers `pub mod` exportés (surface publique)
3. Fichiers internes par taille décroissante (`wc -l *.rs | sort -rn`)
4. Fichiers tests (`*_tests.rs`) — uniquement pour §9

### Étape 0.3 — Cadence et rapports

- **Rapport intermédiaire** : un fichier `docs/audits/code-review/{date}-{crate}.md` par crate, produit dès que le crate est terminé.
- **Rapport final** : `docs/audits/code-review/{date}-final.md` produit après tous les crates.
- **Format de date** : `YYYY-MM-DD` (ISO 8601, ex : `2026-03-07`).
- **Limite par crate** : si un crate génère > 50 constats, arrêter après les 50 plus critiques et noter « audit tronqué — X constats supplémentaires non détaillés ».
- **Ordre des sections** : pour chaque fichier, appliquer les §1-§12 dans l'ordre numérique. Les sections marquées **(AUTO)** dans les pré-requis sont déjà couvertes — ne pas les re-vérifier manuellement.

### Étape 0.4 — Détection automatique du contexte

Si le demandeur ne fournit pas le contexte ci-dessous, l'agent doit le déduire :

```bash
# Type de crate : bin ou lib
grep -q '\[\[bin\]\]' Cargo.toml && echo "bin" || (grep -q 'fn main' src/main.rs 2>/dev/null && echo "bin" || echo "lib")

# MSRV
grep 'rust-version' Cargo.toml || grep 'edition' Cargo.toml

# Runtime async
grep -rn 'tokio::' src/ --include="*.rs" -l | head -1 && echo "tokio"
grep -rn 'async-std::' src/ --include="*.rs" -l | head -1 && echo "async-std"

# Cible réseau (pour §2.9+)
grep -rlE '(TcpListener|TcpStream|UdpSocket|hyper::|reqwest::|axum::|tower::)' src/ --include="*.rs" | head -1 && echo "réseau=oui"

# no_std
grep -q 'no_std' src/lib.rs 2>/dev/null && echo "no_std"

# cargo-deny disponible
command -v cargo-deny >/dev/null && echo "cargo-deny=oui" || echo "cargo-deny=non"
```

Si une information reste indéterminable, **documenter l'hypothèse prise** dans le rapport et continuer.

---

## Contexte attendu de la part du demandeur

Avant de commencer la revue, les informations suivantes doivent être connues. Si le demandeur ne les fournit pas, les déduire avec les commandes de l'étape 0.4, puis **lister les hypothèses prises en tête du rapport**.

| Information | Comment la trouver | Impact sur l'audit |
|-------------|-------------------|-------------------|
| **Type de crate** : `bin` ou `lib` | `grep '\[\[bin\]\]' Cargo.toml` ou présence de `fn main` | §3.4 (anyhow interdit en lib), §9.1 (seuil couverture) |
| **MSRV** | `grep 'rust-version' Cargo.toml` | Disponibilité de certaines API std |
| **Runtime async** | `grep -rn 'tokio\|async-std' src/ --include="*.rs"` | §4 applicable ou non |
| **Cibles de déploiement** | `CLAUDE.md`, `kraft.yaml`, `Cargo.toml` `[target]` | §11.3 (taille binaire critique si unikernel/embedded) |
| **Feature flags actifs** | `cargo metadata --no-deps` → `features` | §8.8, §8.9 |
| **Seuil de couverture** | Demander ou utiliser 80 % par défaut | §9.1 |
| **Politique dépendances** | `ls deny.toml .cargo/audit.toml 2>/dev/null` | §2.5, §11 |
| **Domaine métier** | `CLAUDE.md`, `README.md` | §2.9+ (critères réseau si proxy/serveur) |
| **Crate traite du réseau ?** | `grep -rlE '(TcpListener\|TcpStream\|hyper::\|reqwest::\|axum::\|tower::\|Socket)' src/` | §2.9-2.14, §4 |

---

## Matrice d'applicabilité

Tous les critères ne s'appliquent pas à tous les types de crate. Vérifier cette matrice avant de commencer.

| Section | bin | lib interne | lib publiée | no_std / WASM |
|---------|:---:|:-----------:|:-----------:|:-------------:|
| §1 Correction et sûreté | Oui | Oui | Oui | Oui |
| §2 Sécurité (§2.1-2.8) | Oui | Oui | Oui | Oui |
| §2 Sécurité réseau (§2.9+) | Si réseau | Si réseau | Si réseau | Non |
| §3 Gestion des erreurs | Oui | Oui | Oui | Oui |
| §3.3 Perte de contexte `?` | Oui | Oui | Oui | Oui |
| §3.4 anyhow interdit | N/A | Oui | Oui | Oui |
| §4 Concurrence et async | Oui | Si async | Si async | **Non** |
| §5 Ownership et lifetimes | Oui | Oui | Oui | Oui |
| §6 Performance | Oui | Oui | Oui | Oui |
| §7 Rust idiomatique | Oui | Oui | Oui | Oui |
| §8 Architecture | Oui | Oui | Oui | Oui |
| §9 Tests et observabilité | Oui | Oui | Oui | Partiel* |
| §9.1 Couverture 90% pub métier | Non | Non | Oui | Non |
| §10 Documentation | Oui | Oui | Oui | Oui |
| §10.9 CHANGELOG | Non | Non | Oui | Non |
| §11 Dépendances | Oui | Oui | Oui | Oui |
| §12 Simplification et sur-ingénierie | Oui | Oui | Oui | Oui |

\* *no_std/WASM : les tests sont exécutés sur la cible hôte, pas sur wasm32. La couverture ne s'applique pas si le test runner n'est pas disponible sur la cible.*

---

## Pré-requis automatisés

Exécuter **avant** la revue manuelle. Les critères marqués **AUTO** dans les tableaux sont couverts par ces commandes — l'auditeur ne doit pas les vérifier manuellement.

```bash
# §7.5 — Clippy (AUTO)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# §7.6 — Formatting (AUTO)
cargo fmt --all --check

# §2.1 — Vulnérabilités connues (AUTO)
cargo audit

# §2.5 — Licences (AUTO)
cargo deny check licenses

# §9.7 — Tests (AUTO)
cargo nextest run --workspace

# §10.4 — Documentation (AUTO)
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# §11.1 — Dépendances dupliquées (AUTO)
cargo tree -d

# §11.2 — Dépendances inutilisées (AUTO, si cargo-machete installé)
cargo machete
```

**En cas d'échec d'un pré-requis** :
1. Signaler comme constat **BLOQUANT** si c'est §2.1 (vulnérabilités) ou §9.6 (tests en échec).
2. Signaler comme constat **IMPORTANT** pour les autres (clippy, fmt, doc).
3. **Continuer la revue** dans tous les cas — les pré-requis automatisés ne remplacent pas la revue manuelle, ils la complètent.
4. Ne pas rédiger le bloc Avant/Après pour les pré-requis AUTO — le message d'erreur de la commande suffit.

---

## 1. CORRECTION ET SÛRETÉ — Priorité : CRITIQUE

### Objectifs mesurables

| # | Critère | Seuil | Exception autorisée |
|---|---------|-------|---------------------|
| 1.1 | `unwrap()` / `expect()` hors tests | 0 | Aucune. Y compris dans `build.rs`, `main()`, et les closures passées à `map`. |
| 1.2 | `panic!` / `todo!` / `unimplemented!` explicites hors tests | 0 | Aucune. |
| 1.3 | Panics implicites : indexation `v[i]`, `slice[range]`, division, overflow arithmétique, `FromStr` sans gestion | 0 non documenté | Chaque site documenté par un commentaire `// PANIC-SAFE: <invariant>` prouvant l'impossibilité du panic. |
| 1.4 | Blocs `unsafe` sans commentaire `// SAFETY:` | 0 | Le commentaire doit décrire : (a) quel invariant est maintenu, (b) pourquoi il ne peut pas être violé, (c) quelle pre-condition l'appelant doit respecter. |
| 1.5 | Blocs `unsafe` évitables | 0 | Si une alternative safe existe avec un coût < 10 % en benchmark documenté, l'`unsafe` est rejeté. |
| 1.6 | Branches `_ => unreachable!()` ou `_ =>` wildcard muette dans un `match` sur enum | 0 | Remplacer par un traitement exhaustif de chaque variant pour casser à la compilation si un variant est ajouté. |
| 1.7 | Overflow arithmétique non géré sur des entrées utilisateur | 0 | Utiliser `checked_*`, `saturating_*`, ou `wrapping_*` avec justification. |
| 1.8 | Transmute ou cast `as` tronquant (`u64 as u32`, `usize as u16`, `i64 as u64`) | 0 non justifié | Utiliser `try_from()` / `try_into()` ou justifier le cast avec un commentaire `// TRUNCATION-SAFE:`. |
| 1.9 | `Atomic*` avec `Ordering` incohérent entre load et store sur le même atomic, ou `Relaxed` sur un atomic servant de synchronisation inter-threads | 0 non justifié | Documenter le choix d'ordering par un commentaire `// ORDERING:`. |

### Consignes détaillées

- Remplace tout `unwrap()` / `expect()` par propagation explicite : `?`, `map_err(...)`, `ok_or(...)`, `ok_or_else(...)`. Ne tolère pas `expect("should never happen")` — si ça ne peut pas arriver, prouve-le par le type system.
- Pour chaque indexation `v[i]` : vérifie qu'un bounds check précède, ou remplace par `v.get(i)`.
- Pour chaque division : vérifie que le diviseur est prouvé non-nul, ou utilise `checked_div`.
- Vérifie l'absence de conditions de course :
  - `Mutex` : le guard est-il relâché avant tout `.await` ? (pas de `MutexGuard` tenu à travers un point de suspension async)
  - `RwLock` : pas de promotion read → write sans relâcher le read lock.
  - `Arc` : pas de `Arc::try_unwrap` dans du code concurrent sans preuve que le compteur est bien à 1.
  - `static mut` : interdit. Utiliser `OnceLock`, `LazyLock`, ou `std::sync::atomic`.
- Vérifie que `mem::forget`, `ManuallyDrop`, `MaybeUninit` sont utilisés correctement avec documentation.
- Pour les `Atomic*` : vérifier que `Relaxed` n'est utilisé que pour des compteurs statistiques sans relation happens-before. Tout atomic servant à de la synchronisation (flag d'arrêt, publication de données) doit utiliser `Acquire`/`Release` ou `SeqCst`.

### Détection automatisée (§1)

```bash
# §1.1 — unwrap/expect hors tests
grep -rn '\.unwrap()' --include="*.rs" src/ | grep -v '_tests\.rs'
grep -rn '\.expect(' --include="*.rs" src/ | grep -v '_tests\.rs'

# §1.2 — panic/todo/unimplemented hors tests
grep -rn 'panic!\|todo!\|unimplemented!' --include="*.rs" src/ | grep -v '_tests\.rs'

# §1.3 — indexation directe (faux positifs possibles, vérifier manuellement)
grep -rn '\[.*\]' --include="*.rs" src/ | grep -vE '(\[cfg|#\[|//|_tests\.rs|\.get\()'

# §1.4-1.5 — blocs unsafe
grep -rn 'unsafe' --include="*.rs" src/ | grep -v '_tests\.rs'

# §1.6 — wildcards dans match
grep -rn '_ =>' --include="*.rs" src/ | grep -v '_tests\.rs'

# §1.8 — casts tronquants
grep -rn ' as u32\| as u16\| as u8\| as i32\| as i16\| as i8' --include="*.rs" src/
```

Chaque résultat doit être vérifié manuellement. Les patterns ci-dessus produisent des faux positifs — ils servent de **point de départ**, pas de verdict.

---

## 2. SÉCURITÉ — Priorité : CRITIQUE

### Objectifs mesurables — Critères généraux

| # | Critère | Seuil |
|---|---------|-------|
| 2.1 | `cargo audit` : vulnérabilités connues (RUSTSEC) | 0 advisory non traitée **(AUTO)** |
| 2.2 | Secrets (clés API, tokens, mots de passe) en dur dans le code ou les logs | 0 |
| 2.3 | Données sensibles (PII, secrets) dans les messages d'erreur, `Display`/`Debug` impl, ou les logs (tout niveau) | 0 |
| 2.4 | Entrées utilisateur non validées avant utilisation dans : chemins de fichiers, URLs, regex, headers HTTP | 0 |
| 2.5 | Dépendances avec licence incompatible (vérifiable via `cargo deny check licenses`) | 0 **(AUTO)** |
| 2.6 | `Command::new()` avec des arguments construits par concaténation de strings utilisateur | 0 |
| 2.7 | Désérialisation de données non fiables sans limite de taille/profondeur (`serde_json::from_slice` sur un body HTTP non borné) | 0 |
| 2.8 | Usage de `rand::thread_rng()` ou autre PRNG non-crypto pour des opérations de sécurité (tokens, nonces) | 0 — Utiliser `rand::rngs::OsRng` ou `getrandom`. |

### Objectifs mesurables — Critères réseau et proxy

*Applicables uniquement aux crates traitant du trafic réseau ou des entrées HTTP.*

| # | Critère | Seuil |
|---|---------|-------|
| 2.9 | Comparaison de secrets (tokens, API keys) sans temps constant (`==` au lieu de `constant_time_eq` ou équivalent) | 0 — Un `==` sur un secret est vulnérable aux timing attacks. |
| 2.10 | SSRF : URL de backend construite à partir d'entrée utilisateur sans validation de schéma (`http`/`https` uniquement), d'hôte (pas de `127.0.0.1`, `169.254.x.x`, `[::1]`, `localhost`, `0.0.0.0`, réseaux privés RFC 1918), et de port | 0 |
| 2.11 | Absence de limite de taille sur les headers HTTP entrants (header name, header value, nombre total de headers) | 0 — Un header de taille illimitée est un vecteur de déni de service. |
| 2.12 | Opération réseau (connect, read, write, TLS handshake) sans timeout | 0 — Toute socket sans timeout est un risque Slowloris. |
| 2.13 | Injection CRLF : header HTTP name ou value construit par concaténation de string utilisateur sans validation/échappement de `\r\n` | 0 |
| 2.14 | Sandbox WASM : accès mémoire host hors du buffer alloué au plugin, ou appel host function sans validation des pointeurs/longueurs passés par le plugin | 0 |

### Consignes détaillées

- Vérifie que les secrets sont chargés depuis des variables d'environnement, un vault, ou des fichiers avec permissions restreintes — jamais depuis le code source.
- Vérifie que les implémentations de `Display` et `Debug` sur les types contenant des secrets redactent les champs sensibles (afficher `***` ou omettre).
- Vérifie que les chemins de fichiers construits à partir d'entrées utilisateur passent par une canonicalisation et un confinement (pas de path traversal `../../etc/passwd`).
- Vérifie la configuration TLS : pas de `danger_accept_invalid_certs(true)` en production.
- Vérifie les limites de taille sur les entrées réseau (`Content-Length`, body size limits, profondeur de parsing JSON/XML).
- Vérifie que `RUST_LOG` ne peut pas être configuré par un utilisateur non privilégié pour leaker des secrets en `TRACE`.
- Pour un proxy : vérifier que les headers `Host`, `X-Forwarded-For`, `X-Forwarded-Proto` ne sont pas trustés aveuglément depuis le client.

### Détection automatisée (§2)

```bash
# §2.2 — secrets en dur (mots-clés suspects)
grep -rnEi '(password|secret|token|api_key|apikey)\s*=\s*"[^"]{4,}"' --include="*.rs" src/

# §2.3 — secrets dans les logs/Display
grep -rn 'tracing::\|log::\|println!\|eprintln!\|format!' --include="*.rs" src/ | grep -iE '(secret|password|token|key)'

# §2.6 — Command::new avec concaténation
grep -rn 'Command::new' --include="*.rs" src/

# §2.7 — désérialisation sans limite
grep -rn 'serde_json::from_slice\|serde_json::from_str\|serde_json::from_reader' --include="*.rs" src/

# §2.9 — comparaison de secrets non constant-time
grep -rn '==.*secret\|secret.*==' --include="*.rs" src/

# §2.10 — SSRF : construction d'URL
grep -rn 'format!.*http\|Url::parse' --include="*.rs" src/

# §2.12 — opérations réseau sans timeout
grep -rn '\.connect(\|TcpStream::' --include="*.rs" src/

# §2.13 — injection CRLF
grep -rn 'HeaderValue::from_str\|header.*format!' --include="*.rs" src/
```

---

## 3. GESTION DES ERREURS — Priorité : HAUTE

### Objectifs mesurables

| # | Critère | Seuil | Précision |
|---|---------|-------|-----------|
| 3.1 | `let _ = result;` sans commentaire | 0 | Chaque suppression intentionnelle annotée `// intentionnellement ignoré : <raison>`. |
| 3.2 | Erreurs I/O, FFI, réseau non converties en type d'erreur du domaine | 0 | Chaque frontière (I/O, FFI, HTTP, DB) doit avoir une conversion explicite. |
| 3.3 | Propagation `?` qui perd de l'information de contexte (l'erreur résultante ne permet pas d'identifier l'opération tentée et la ressource impliquée) | 0 | Ne s'applique pas si le type d'erreur cible contient déjà l'information (ex : variant `thiserror` avec champs). Utiliser `map_err`, un variant dédié, ou `.context()` (si `anyhow` est autorisé dans le crate). |
| 3.4 | `anyhow::Error` dans une bibliothèque (`lib.rs` ou crate publiée) | 0 | Bibliothèques : `thiserror` obligatoire. `anyhow` toléré uniquement dans les binaires. |
| 3.5 | Type d'erreur "fourre-tout" unique pour tout un binaire | ≤ 1 par crate bin | Si le binaire a plusieurs modules, chaque module devrait avoir son type d'erreur convertible vers le type racine. |
| 3.6 | Variants d'erreur sans champ de contexte | 0 pour les erreurs I/O/réseau | Ex : `IoError { source: io::Error }` est insuffisant → ajouter `path: PathBuf` ou `operation: &'static str`. |
| 3.7 | `Box<dyn Error>` comme type de retour dans du code non-prototype | 0 | Remplacer par un enum typé ou `anyhow::Error` (binaires). |
| 3.8 | Retry logic sans backoff exponentiel ou sans limite de tentatives | 0 | Toute boucle de retry doit avoir : un max de tentatives, un backoff (exponentiel ou jitté), et un log à chaque tentative. |

### Consignes détaillées

- Vérifie que les erreurs FFI (`c_int`, codes errno) sont converties en `Result<T, E>` avec un variant spécifique.
- Vérifie que `?` ne perd pas d'information de contexte. Le critère est la **qualité du message d'erreur résultant**, pas la présence mécanique de `.context()`.
- Les erreurs loguées doivent l'être au bon niveau : `warn!` pour récupérable, `error!` pour fatal. Pas de `info!` pour une erreur.
- Vérifie l'absence de `eprintln!` / `println!` pour les erreurs (utiliser le framework de logging).
- Chaque `From<E>` implicite (via `?`) devrait préserver la chaîne de causalité (`#[source]` ou `#[from]` avec `thiserror`).

---

## 4. CONCURRENCE ET ASYNC — Priorité : HAUTE

*Non applicable aux crates `no_std` / WASM. Voir matrice d'applicabilité.*

### Objectifs mesurables

| # | Critère | Seuil |
|---|---------|-------|
| 4.1 | `MutexGuard` (sync) tenu à travers un `.await` | 0 |
| 4.2 | Appel bloquant (`std::fs`, `std::net`, `thread::sleep`, `Mutex::lock` de `std`) dans un contexte async sans `spawn_blocking` | 0 — Exception : `std::env::var()`, accès à un `std::sync::Mutex` protégeant un champ mémoire sans I/O (lock + read/write + unlock sans `.await`), et opérations purement CPU sur < 64 octets. |
| 4.3 | `tokio::spawn` sans gestion du `JoinHandle` (fire-and-forget silencieux) | 0 sans justification explicite |
| 4.4 | `tokio::spawn` sur un Future non `Send` (compilation échouera, mais vérifier les bornes) | 0 |
| 4.5 | Task annulable (`select!`, `timeout`) sans nettoyage des ressources (cancellation safety) | 0 non documenté |
| 4.6 | Canal (`mpsc`, `broadcast`, `watch`) sans stratégie de back-pressure ou de taille bornée | 0 — tout canal non borné doit être justifié. |
| 4.7 | `Arc<Mutex<T>>` acquis dans un ordre non documenté entre plusieurs locks | 0 — documenter l'ordre d'acquisition global. |
| 4.8 | `Rc` ou `RefCell` dans du code async multi-thread | 0 — utiliser `Arc` / `tokio::sync::Mutex`. |
| 4.9 | `std::sync::Mutex` dans du code async | 0 non justifié — `std::sync::Mutex` est acceptable si le lock dure < 1µs et ne contient pas de `.await`. Sinon préférer `tokio::sync::Mutex`. Documenter le choix. |
| 4.10 | `.lock().unwrap()` sur un Mutex empoisonné sans stratégie de récupération | 0 — documenter la stratégie (ignorer le poison, propager, ou panic intentionnel). |

### Consignes détaillées

- Pour chaque `.await` : vérifie qu'aucune ressource synchrone (guard, file handle, etc.) n'est tenue à travers le point de suspension.
- Pour chaque `select!` : vérifie que chaque branche est cancellation-safe. Si un Future n'est pas cancellation-safe, le documenter ou utiliser `tokio::pin!` + boucle.
- Pour chaque `spawn` : vérifie que les panics de la task sont gérés (via `JoinHandle::await` ou un panic hook global).
- Vérifie que les Futures sont `Send + 'static` quand nécessaire (pas de références locales capturées dans un `spawn`).
- Vérifie que `graceful shutdown` est implémenté : `CancellationToken`, `watch` channel, ou `signal::ctrl_c()` + propagation aux tasks.

### Détection automatisée (§4)

```bash
# §4.1 — MutexGuard tenu à travers .await (heuristique : lock() et .await dans la même fonction)
grep -rn '\.lock()' --include="*.rs" src/ | while read line; do
  file=$(echo "$line" | cut -d: -f1)
  lineno=$(echo "$line" | cut -d: -f2)
  # Vérifier si un .await existe dans les 20 lignes suivantes
  sed -n "$((lineno)),$(($lineno+20))p" "$file" | grep -q '\.await' && echo "SUSPECT: $line"
done

# §4.2 — Appels bloquants dans du code async
grep -rn 'std::fs::\|std::net::\|thread::sleep' --include="*.rs" src/ | grep -v '_tests\.rs'

# §4.3 — tokio::spawn sans JoinHandle
grep -rn 'tokio::spawn' --include="*.rs" src/ | grep -v 'let.*='

# §4.6 — canaux non bornés
grep -rn 'unbounded_channel\|channel()' --include="*.rs" src/

# §4.8 — Rc/RefCell dans du code async
grep -rn 'Rc<\|RefCell<' --include="*.rs" src/ | grep -v '_tests\.rs'
```

---

## 5. OWNERSHIP, BORROWING ET LIFETIMES — Priorité : MOYENNE

### Objectifs mesurables

| # | Critère | Seuil |
|---|---------|-------|
| 5.1 | `.clone()` sur un type `Copy` | 0 — utiliser la copie implicite. |
| 5.2 | `.clone()` là où une référence `&T` suffit (le récepteur n'a pas besoin d'ownership) | 0 non justifié |
| 5.3 | `String` en paramètre là où `&str` suffit ; `Vec<T>` en paramètre là où `&[T]` suffit | 0 — sauf si la fonction a besoin d'ownership (stockage dans une struct, envoi dans un thread). Voir aussi §6.3. |
| 5.4 | `Rc<RefCell<T>>` sans justification documentée | 0 |
| 5.5 | Lifetimes explicites génériques (`'a`, `'b`) quand il y en a > 1 dans une même signature | SUGGESTION — nommer sémantiquement (`'conn`, `'buf`, `'query`) améliore la lisibilité mais n'est pas obligatoire. |
| 5.6 | Lifetime elision incorrecte (ex : retour de référence sans que la lifetime du paramètre soit évidente) | 0 — si la règle d'elision ne s'applique pas clairement, la lifetime doit être explicite. |
| 5.7 | `'static` sur une borne de trait (`T: 'static`) sans que ce soit strictement nécessaire | 0 non justifié — `'static` est souvent trop restrictif et empêche l'emprunt. |
| 5.8 | `.to_owned()` / `.to_string()` en chaîne (ex : `format!("...").to_string()`) — redondant | 0 |

---

## 6. PERFORMANCE — Priorité : MOYENNE

### Objectifs mesurables

| # | Critère | Seuil |
|---|---------|-------|
| 6.1 | Allocation (`Vec::new()`, `String::new()`, `format!()`, `Box::new()`, `.collect()`) dans une boucle chaude (= boucle exécutée par requête/connexion entrante, ou boucle `loop`/`while` du runtime principal) | 0 sans commentaire justifiant l'impossibilité de la sortir. |
| 6.2 | `.collect::<Vec<_>>()` suivi immédiatement d'un `.iter()` / `.into_iter()` / autre itérateur | 0 — fusionner en une seule chaîne d'itérateur. |
| 6.3 | `String` par valeur en paramètre de fonction là où `&str` suffit | 0 — voir §5.3 pour le détail. Ne reporter qu'une seule fois. |
| 6.4 | `Box<dyn Trait>` dans un chemin chaud (= appelé à chaque requête/connexion, ou dans une boucle du runtime) sans benchmark justifiant le dispatch dynamique | 0 — préférer un type générique `impl Trait` ou un enum dispatch. Heuristique : si la fonction est dans le pipeline de traitement d'une requête, c'est un chemin chaud. |
| 6.5 | Type `Copy` > 16 octets passé par valeur dans un chemin chaud | 0 — passer par `&T`. Ne s'applique pas aux types `!Copy` (le compilateur fait du move, pas de copie). |
| 6.6 | `HashMap` / `BTreeMap` avec un type de clé coûteux à hasher/comparer sans benchmark | 0 non documenté — envisager `ahash`, `FxHashMap`, ou une clé plus légère. |
| 6.7 | `Vec` initialisé sans `with_capacity()` quand la taille est connue ou estimable à l'avance | 0 dans les chemins chauds. |
| 6.8 | `format!()` utilisé uniquement pour convertir un seul élément (ex : `format!("{}", x)` au lieu de `x.to_string()`) | 0 |
| 6.9 | `.clone()` dans une boucle chaude sur un type non-`Copy` sans justification | 0 |
| 6.10 | Absence de `#[inline]` sur les fonctions critiques d'une bibliothèque appelées à travers les frontières de crate | Signaler comme SUGGESTION si applicable. |

---

## 7. RUST IDIOMATIQUE — Priorité : MOYENNE

### Objectifs mesurables

| # | Critère | Seuil |
|---|---------|-------|
| 7.1 | `match` sur `Option`/`Result` à un seul bras d'intérêt (2 bras dont un `_ => ()`) | 0 — utiliser `if let`, `.map()`, `.and_then()`, `.unwrap_or_else()`. |
| 7.2 | Boucle `for i in 0..v.len() { v[i] }` remplaçable par `.iter()` / `.enumerate()` | 0 |
| 7.3 | Derives manquants (`Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`) là où l'implémentation manuelle serait identique | 0 — en particulier, `Debug` est obligatoire sur tout type exposé publiquement. |
| 7.4 | Violation des conventions de nommage Rust (`snake_case`, `PascalCase`, `SCREAMING_SNAKE_CASE`) | 0 |
| 7.5 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 warning **(AUTO)** |
| 7.6 | `cargo fmt --check` | 0 diff **(AUTO)** |
| 7.7 | `impl Default` manuel là où `#[derive(Default)]` suffit | 0 |
| 7.8 | Constructeur nommé `new()` qui ne retourne pas `Self` | 0 |
| 7.9 | `impl Display` sans `impl Error` sur un type d'erreur | 0 |
| 7.10 | `.into_iter()` explicite là où l'itérateur `for x in collection` suffit | 0 — sauf si l'explicite améliore la lisibilité dans un contexte complexe. |
| 7.11 | `return` explicite à la fin d'une fonction (non-idiomatique en Rust) | 0 — utiliser l'expression finale sans `return`. |
| 7.12 | Fonction retournant `Result` ou `bool` important sans `#[must_use]` | SUGGESTION — `#[must_use]` évite les appels silencieusement ignorés. |

---

## 8. ARCHITECTURE ET MAINTENABILITÉ — Priorité : MOYENNE

### Objectifs mesurables

| # | Critère | Seuil |
|---|---------|-------|
| 8.1 | Fonction > 60 lignes (mesurées par `wc -l` entre `fn` et le `}` fermant, hors lignes vides et commentaires) | 0 — découper en sous-fonctions nommées. Exceptions : (a) blocs `match` exhaustifs sur des enums de routing (routeur HTTP) comptés comme une unité logique, (b) fonctions de construction/initialisation dont le corps est une liste de champs. |
| 8.2 | Fonction avec > 5 paramètres | 0 — utiliser un struct builder ou un struct de config. |
| 8.3 | `pub` sur un item utilisé uniquement dans le même module ou la même crate | 0 — utiliser `pub(crate)`, `pub(super)`, ou supprimer. |
| 8.4 | Profondeur d'imbrication > 3 niveaux (`if` / `match` / `loop` / `for`) | 0 — extraire en sous-fonction, utiliser early return, ou `?`. |
| 8.5 | Trait avec > 7 méthodes | 0 — décomposer en traits plus focalisés (ISP). |
| 8.6 | Identifiant sémantique primitif nu (`u64` pour un ID, `String` pour un chemin, `f64` pour un montant) dans les **signatures publiques inter-crates** | 0 — utiliser un newtype. Ne s'applique pas aux primitifs utilisés en interne dans un seul crate. |
| 8.7 | Couplage circulaire entre modules (module A importe module B qui importe A) | 0 — extraire les types partagés dans un module commun. |
| 8.8 | Feature flag de crate non testé en CI (combinaisons non couvertes) | Signaler — chaque combinaison de features activées/désactivées doit compiler. |
| 8.9 | `cfg(target_os)` ou `cfg(feature)` sans test correspondant dans la CI | 0 non signalé. |
| 8.10 | Dépendance sur un comportement non garanti par la doc std (ex : ordre d'itération d'un `HashMap`) | 0 |

---

## 9. TESTS ET OBSERVABILITÉ — Priorité : MOYENNE

### Objectifs mesurables

| # | Critère | Seuil |
|---|---------|-------|
| 9.1 | Couverture de code (via `cargo llvm-cov` ou `cargo tarpaulin`) | ≥ 80 % de couverture de lignes sur l'ensemble du workspace. Pour les crates bibliothèques publiées : ≥ 90 % sur les fonctions `pub` contenant des branchements (`if`, `match`, `?`, boucles) — les derives auto, accesseurs à une ligne (getter/setter sans logique), et impls `From`/`Into` triviales sont exemptés du calcul. |
| 9.2 | Fonction publique sans test | 0 — au minimum 1 test nominal + 1 test de cas limite. |
| 9.3 | Test sans assertion (`#[test] fn test_foo() { foo(); }`) | 0 |
| 9.4 | Test ignoré (`#[ignore]`) sans commentaire daté + ticket | 0 |
| 9.5 | Événement métier (démarrage, arrêt, erreur critique, transaction validée) sans log structuré `INFO`/`ERROR` | 0 — chaque événement doit avoir au minimum un champ structuré identifiant (`tracing::info!(user_id = %id, "commande validée")`). |
| 9.6 | `cargo test --all-features` : tests en échec | 0 **(AUTO)** |
| 9.7 | Tests déterministes : dépendance à l'heure système, à l'ordre d'exécution, à l'accès réseau, ou au filesystem sans mock/abstraction | 0 — les tests doivent pouvoir tourner en parallèle et hors-ligne. |
| 9.8 | Test d'intégration sans cleanup (fichiers temporaires, connexions DB, ports réseau) | 0 — utiliser `tempfile`, RAII, ou un `Drop` pour le cleanup. |
| 9.9 | Métriques : les compteurs et histogrammes critiques (latence, taux d'erreur, saturation de files) sont-ils exposés ? | Signaler comme SUGGESTION si absent dans un service. |
| 9.10 | Health check / readiness probe pour les services | Signaler comme SUGGESTION si absent. |

---

## 10. DOCUMENTATION — Priorité : BASSE

### Objectifs mesurables

| # | Critère | Seuil |
|---|---------|-------|
| 10.1 | Item `pub` ou `pub(crate)` sans doc-comment `///` | 0 — au moins une phrase décrivant le contrat. |
| 10.2 | Exemples dans les doc-comments qui ne compilent pas (`cargo test --doc`) | 0 |
| 10.3 | `// TODO` ou `// FIXME` sans référence ticket (`// TODO(#123): ...`) et sans date | 0 |
| 10.4 | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | 0 warning **(AUTO)** |
| 10.5 | Fonction `pub` avec des pré-conditions non documentées (ex : "le vecteur ne doit pas être vide") | 0 — documenter dans une section `# Panics` ou `# Errors` du doc-comment. |
| 10.6 | Fonction `pub` retournant `Result` sans section `# Errors` listant les cas d'erreur | 0 |
| 10.7 | Fonction `pub unsafe` sans section `# Safety` dans le doc-comment | 0 |
| 10.8 | README ou module-level doc (`//!`) absent à la racine de chaque crate dans un workspace | 0 |
| 10.9 | `CHANGELOG.md` ou mécanisme de suivi des changements absent pour une bibliothèque publiée | Signaler comme SUGGESTION. Non applicable aux crates internes. |

---

## 11. DÉPENDANCES — Priorité : BASSE

### Objectifs mesurables

| # | Critère | Seuil |
|---|---------|-------|
| 11.1 | Dépendances dupliquées (`cargo tree -d`) : deux versions de la même crate | 0 non justifié **(AUTO)** — chaque doublon gonflé le binaire et la surface d'attaque. Justifier par un commentaire dans `Cargo.toml` si inévitable (ex : transitive). |
| 11.2 | Dépendances inutilisées (`cargo machete` ou revue manuelle des `use`) | 0 **(AUTO si outillage disponible)** |
| 11.3 | Dépendance lourde (> 500 KB compilée ou > 20 dépendances transitives) pour une fonctionnalité mineure | Signaler comme SUGGESTION — envisager une alternative plus légère ou une implémentation interne. Critique pour les unikernels et l'embedded. |
| 11.4 | Dépendance sans maintenance (dernier commit > 12 mois, issues critiques ouvertes non traitées) | Signaler comme SUGGESTION |
| 11.5 | Feature flags de dépendances activant des fonctionnalités inutilisées (ex : `serde/derive` activé par défaut alors que seul `serde/std` est nécessaire) | 0 — `default-features = false` + activation explicite des features nécessaires. |

---

## 12. SIMPLIFICATION ET SUR-INGÉNIERIE — Priorité : HAUTE

La sur-ingénierie est un défaut aussi grave que la sous-ingénierie. Du code inutilement complexe est plus difficile à auditer, plus coûteux à maintenir, et cache des bugs derrière des couches d'abstraction. Le bon code est le code le plus simple qui résout le problème réel — pas le problème hypothétique de demain.

### Objectifs mesurables — Abstractions prématurées

| # | Critère | Seuil |
|---|---------|-------|
| 12.1 | Trait avec une seule implémentation **et** aucun plan de test avec mock | 0 — utiliser directement le type concret. Un trait n'a de valeur que s'il est implémenté au moins deux fois (type réel + mock de test) ou s'il sert de frontière de crate publiée. |
| 12.2 | Type générique (`<T: Trait>`) instancié avec un seul type concret dans tout le codebase | 0 non justifié — remplacer par le type concret. La généricité se justifie par l'usage, pas par la « flexibilité future ». Exception : si le type vit dans une bibliothèque publiée dont les consommateurs ne sont pas connus. |
| 12.3 | Couche d'indirection sans logique propre (struct wrapper qui délègue tous ses appels à l'inner sans transformation, module `mod.rs` qui ne fait que re-exporter un seul sous-module) | 0 — supprimer la couche, exposer directement. |
| 12.4 | Builder pattern pour un struct ≤ 4 champs sans invariants de construction | 0 — utiliser la construction directe `Struct { field: value }` ou `Default` + mutation. Le builder se justifie quand : (a) il y a des validations à l'init, (b) > 4 champs, ou (c) API publique de bibliothèque. |
| 12.5 | Enum avec un seul variant utilisé | 0 — remplacer par le type du variant directement. Un enum à 1 variant est un newtype déguisé. |
| 12.6 | Feature flag pour du code qui n'a jamais été désactivé en production | Signaler — les features inutilisées alourdissent la matrice de test et la CI. Si un feature flag n'a jamais été `false` en prod, le code qu'il garde devrait être le chemin par défaut sans condition. |

### Objectifs mesurables — Complexité inutile

| # | Critère | Seuil |
|---|---------|-------|
| 12.7 | Code mort : fonction, méthode, variant d'enum, champ de struct ou module entier jamais appelé hors des tests | 0 — supprimer. `#[allow(dead_code)]` est un signal d'alerte, pas une solution. Exception : items `pub` d'une bibliothèque destinée à des consommateurs externes. |
| 12.8 | Duplication de logique identique dans ≥ 3 sites **sans** extraction en fonction commune | 0 — factoriser. Attention : ne pas factoriser si les 3 sites divergeront probablement (évaluer le couplage). |
| 12.9 | Duplication de logique dans 2 sites avec extraction prématurée en abstraction (helper, trait, macro) | 0 — trois occurrences similaires sont le seuil minimum pour factoriser. Deux lignes identiques ne justifient pas une abstraction. Préférer la duplication à la mauvaise abstraction. |
| 12.10 | Gestion d'erreurs, fallback ou validation pour un scénario qui ne peut pas se produire (ex : `if x < 0` sur un `u32`, check de nullité sur un type non-Option, `match` avec branche « impossible ») | 0 — supprimer le code défensif inutile. Si l'invariant est garanti par le type system, le code qui le vérifie est du bruit. |
| 12.11 | Paramètre de configuration pour une valeur qui ne change jamais en pratique (ex : constante extraite en config « au cas où ») | Signaler — une constante est plus claire qu'un paramètre inutilisé. Si la valeur n'a jamais été modifiée en production, la rendre `const`. |
| 12.12 | Macro déclarative ou procédurale remplaçable par une fonction générique ou un trait | 0 — les macros sont plus difficiles à débugger, à documenter et à tester. Ne les utiliser que si le type system ne suffit pas (génération de code, DSL, variadic). |
| 12.13 | Conversion intermédiaire inutile : `A → B → C` quand `A → C` est possible directement | 0 — chaque conversion alloue, copie, et ajoute un point de failure. |

### Objectifs mesurables — État et structure

| # | Critère | Seuil |
|---|---------|-------|
| 12.14 | Deux structures contenant les mêmes champs (ou un sous-ensemble significatif) avec des accès parallèles aux mêmes données | 0 — unifier en une seule source de vérité. Symptôme classique : `StateA.field` et `StateB.inner.field` pointent vers le même `Arc`. |
| 12.15 | Module ou crate dont la seule raison d'être est « on en aura peut-être besoin plus tard » | 0 — YAGNI. Le coût de maintenance d'un module vide ou squelettique est supérieur au coût de le créer le jour où il sera nécessaire. |
| 12.16 | Hiérarchie de types à > 3 niveaux d'imbrication (`Struct<Arc<Mutex<Vec<Option<T>>>>>`) | 0 non justifié — introduire un type alias nommé ou simplifier la structure. Si l'imbrication existe, c'est souvent que la responsabilité du type est mal découpée. |
| 12.17 | Struct avec > 10 champs sans décomposition en sous-structs cohérentes | Signaler — un struct à 10+ champs est souvent un god object. Évaluer si des groupes de champs forment des sous-domaines indépendants. |

### Consignes détaillées

- **Test du « et si on supprimait ? »** : pour chaque abstraction (trait, generic, wrapper, builder, enum), se demander : « que se passerait-il si on la remplaçait par le cas concret ? ». Si la réponse est « rien ne casse sauf des tests de l'abstraction elle-même », l'abstraction est inutile.
- **Règle des 3** : ne factoriser que quand 3 occurrences réelles existent. 2 doublons sont tolérés. « WET is better than the wrong DRY. »
- **Complexité accidentelle vs essentielle** : la complexité essentielle vient du domaine (parsing TLS, matching de règles). La complexité accidentelle vient de l'architecture (couches, indirections, conversions). L'auditeur doit challenger toute complexité accidentelle.
- **Dead code** : un `pub fn` sans appelant dans le workspace est suspect. Vérifier via `grep -rn "nom_fonction" --include="*.rs"`. Les items `pub` d'un crate `lib` destiné à des consommateurs externes sont exemptés.
- **Double état** : si deux structures partagent des `Arc` vers les mêmes données, c'est un signe de duplication structurelle. L'une des deux est de trop. Vérifier que chaque donnée a une seule source de vérité et un seul chemin d'accès.

### Détection automatisée (§12)

```bash
# §12.1 — Traits avec une seule implémentation
for trait_name in $(grep -rn '^pub trait ' --include="*.rs" src/ | sed 's/.*pub trait \([A-Za-z_]*\).*/\1/'); do
  count=$(grep -rn "impl.*$trait_name" --include="*.rs" src/ | grep -v '_tests\.rs' | wc -l)
  [ "$count" -le 1 ] && echo "SINGLE-IMPL: $trait_name ($count impl)"
done

# §12.3 — Wrappers sans logique (structs avec un seul champ qui délèguent)
grep -rn 'pub struct.*(' --include="*.rs" src/ | grep -v '_tests\.rs'

# §12.5 — Enums avec un seul variant
grep -rn '^pub enum' --include="*.rs" -A 5 src/ | grep -B1 '^}' | grep -v '^--$'

# §12.7 — Code mort (fonctions pub non appelées dans le workspace)
for fn_name in $(grep -rn 'pub fn ' --include="*.rs" src/ | sed 's/.*pub fn \([a-z_]*\).*/\1/' | sort -u); do
  count=$(grep -rn "$fn_name" --include="*.rs" src/ | wc -l)
  [ "$count" -le 1 ] && echo "DEAD?: pub fn $fn_name (1 occurrence = déclaration seule)"
done

# §12.10 — Checks impossibles sur des types non-Option
grep -rn 'if.*\.is_none()\|if.*\.is_some()' --include="*.rs" src/ | grep -v '_tests\.rs'

# §12.14 — Double état (structures avec les mêmes champs Arc)
grep -rn 'Arc<' --include="*.rs" src/ | grep 'pub ' | sed 's/.*Arc<\(.*\)>.*/\1/' | sort | uniq -c | sort -rn | head -20

# §12.16 — Imbrication profonde de types
grep -rnE 'Arc<.*Mutex<.*Vec<|Arc<.*RwLock<.*HashMap<' --include="*.rs" src/

# §12.17 — Structs avec > 10 champs
grep -rn '^pub struct' --include="*.rs" -A 20 src/ | awk '/^pub struct/{name=$0; count=0} /pub /{count++} /^}/{if(count>10) print count" champs: "name}'
```

**Note** : ces commandes produisent des faux positifs. Chaque résultat doit être vérifié en lisant le code source. Elles servent de pré-filtrage pour orienter l'attention de l'auditeur.

---

## FORMAT DE RÉPONSE

### Pour les constats BLOQUANT et IMPORTANT

Format complet obligatoire :

```
**[PRIORITÉ]** — `chemin/fichier.rs` L42-L58
**Critère violé :** §X.Y — <intitulé exact du critère>
**Problème :** Description factuelle, sans ambiguïté.
**Avant :**
\`\`\`rust
// code actuel
\`\`\`
**Après :**
\`\`\`rust
// code corrigé ou refactorisé
\`\`\`
**Justification :** Pourquoi ce changement est nécessaire en production.
Impact si non corrigé : <conséquence concrète — crash, fuite mémoire, vulnérabilité, data loss, etc.>
```

### Pour les constats MINEUR et SUGGESTION

Format allégé autorisé :

```
**[MINEUR]** — §X.Y — <intitulé court>
Fichiers concernés : `foo.rs` (L12, L45, L89), `bar.rs` (L3, L67)
Action : <correction en une phrase>
```

### Fichiers de sortie

- Rapports intermédiaires : `docs/audits/code-review/{YYYY-MM-DD}-{crate}.md` (ex : `2026-03-07-proxy-core.md`)
- Rapport final : `docs/audits/code-review/{YYYY-MM-DD}-final.md`

### Échelle de priorité

| Priorité | Signification | Bloque la mise en prod ? |
|----------|---------------|--------------------------|
| **BLOQUANT** | Bug, crash, vulnérabilité, perte de données, UB | Oui — ne pas déployer. |
| **IMPORTANT** | Risque élevé de problème en conditions réelles (perf, concurrence, observabilité insuffisante) | Oui — sauf dérogation documentée avec plan de correction daté. |
| **MINEUR** | Non-idiomatique, dette technique, maintenabilité dégradée | Non — mais créer un ticket. |
| **SUGGESTION** | Amélioration de confort, optimisation marginale, polish | Non — à la discrétion de l'équipe. |

### Tableau de conformité récapitulatif

À produire systématiquement en fin de revue, dans le fichier d'audit pour chaque crate :

| Section | Critères évalués | Conformes | Non conformes | Bloquants |
|---------|-----------------|-----------|---------------|-----------|
| 1. Correction et sûreté | X | X | X | X |
| 2. Sécurité | X | X | X | X |
| 3. Gestion des erreurs | X | X | X | X |
| 4. Concurrence et async | X | X | X | X |
| 5. Ownership et lifetimes | X | X | X | X |
| 6. Performance | X | X | X | X |
| 7. Rust idiomatique | X | X | X | X |
| 8. Architecture | X | X | X | X |
| 9. Tests et observabilité | X | X | X | X |
| 10. Documentation | X | X | X | X |
| 11. Dépendances | X | X | X | X |
| 12. Simplification et sur-ingénierie | X | X | X | X |
| **TOTAL** | **X** | **X** | **X** | **X** |

### Verdict final

Un seul parmi :

- **PRÊT POUR LA PRODUCTION** — 0 bloquant, 0 important. Code déployable en l'état.
- **PRÊT SOUS CONDITIONS** — 0 bloquant, IMPORTANT restants avec plan de correction daté. Déploiement possible avec suivi. Les IMPORTANT ne doivent pas être dans §1 (Correction) ou §2 (Sécurité).
- **NÉCESSITE DES CORRECTIONS** — ≥ 1 bloquant, ou IMPORTANT dans §1/§2. Retour en développement.
- **REFACTORING PROFOND RECOMMANDÉ** — Problèmes architecturaux ou de design rendant les corrections ponctuelles insuffisantes. Planifier un sprint dédié.

### Résumé exécutif

Termine par un résumé de 3 à 5 phrases à destination d'un lead technique qui ne lira pas le détail :
- Nombre de problèmes par priorité.
- Les 3 risques les plus critiques.
- Estimation de l'effort de correction (en jours-développeur).

Ce résumé exécutif sera dans `docs/audits/code-review/{date}-final.md`. Il proposera en outre les évolutions architecturales dans le code, l'évolution des interfaces, des types, etc.
