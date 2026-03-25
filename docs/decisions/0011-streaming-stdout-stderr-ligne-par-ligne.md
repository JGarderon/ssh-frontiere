# ADR 0011 — Streaming stdout/stderr ligne par ligne

**Date** : 2026-03-18
**Statut** : Proposée
**Participants** : Julien (BO), Claude (PM/Tech Lead), Agents Claude Code
**Réf.** : ADR 0006 (protocole d'entêtes — préfixe `>>` pour les réponses), ADR 0003 (contrat d'interface JSON — format à champs fixes, distinction `null` vs `""`)
**Voir aussi** : TODO-030, ADR 0008 (tags), ADR 0009 (arguments nommés)

---

## Contexte

SSH Frontière bufferise actuellement l'intégralité de stdout/stderr d'une commande exécutée, puis renvoie le tout dans le JSON de réponse :

```
>> {"command":"gitlab backup-full","status_code":0,"status_message":"executed","stdout":"tout le contenu\n...","stderr":""}
```

Pour les commandes longues (backup GitLab ~30s, purge Mastodon), le client ne voit rien pendant l'exécution puis reçoit un bloc monolithique.

### Problème concret (retour migration Rundeck → Forgejo Actions)

La migration vers Forgejo Actions a rendu ce comportement bloquant :

1. **Pas de retour en temps réel** : les logs Forgejo Actions n'affichent rien pendant l'exécution — les opérateurs voient un step « en cours » pendant 30 secondes sans information
2. **Diagnostic impossible** : si un script échoue au milieu, on ne sait pas à quelle étape — il faut relancer manuellement pour observer
3. **Timeout aveugle** : impossible de distinguer une commande qui travaille activement d'une commande bloquée sur un lock ou un I/O mort

### État actuel de l'implémentation

- `dispatch.rs:execute_command()` : capture stdout/stderr via `Stdio::piped()`, attend la fin du processus, lit les pipes d'un bloc avec `read_to_string()`
- `chain.rs:execute_single_command()` : reçoit `(code, stdout, stderr)`, construit `Response::executed()`, envoie via `write_response()`
- `protocol.rs:write_response()` : écrit `>> {json}\n` sur stdout
- `output.rs:Response` : sérialise les champs `stdout: Option<String>` et `stderr: Option<String>` dans le JSON

Le bottleneck est dans `dispatch.rs` : toute la sortie est accumulée en RAM avant d'être envoyée.

### Note sur le format JSON

L'ADR 0003 définit un « format fixe à 4 champs » (`status_code`, `status_message`, `stdout`, `stderr`). L'implémentation actuelle (alignement 003, Phase 3) a ajouté un 5ème champ `command` qui identifie la commande exécutée. La struct `Response` dans `output.rs` sérialise ces 5 champs. Cette ADR s'appuie sur le format à 5 champs tel qu'implémenté.

### Note sur les préfixes de direction serveur

L'ADR 0006 §1 définit abstraitement 4 préfixes (`+`, `#`, `$`, `>`). L'implémentation distingue la direction via un suffixe `>` : les lignes serveur → client utilisent `+>`, `#>`, `>>` (le `>` additionnel marque l'origine serveur). Cette convention n'est pas formalisée dans l'ADR 0006 mais est établie dans le code depuis la Phase 3. Cette ADR s'appuie sur les préfixes d'implémentation (`>>` pour les réponses, pas `>` abstrait).

---

## Options

### Option A — Streaming avec trois préfixes distincts

Trois préfixes pour séparer le contenu streamé de la réponse finale :

- `>> ` : ligne de stdout (streamée en temps réel)
- `>>! ` : ligne de stderr (streamée en temps réel)
- `>>> ` : réponse JSON finale (anciennement `>> `)

Avantage : lisible, explicite, chaque flux est identifiable sans ambiguïté. Le nombre de chevrons indique la nature : 2 pour le contenu brut, 3 pour le résultat structuré.

### Option B — Streaming via commentaires serveur (#>)

Réutiliser le préfixe `#>` existant pour streamer le contenu :

- `#> [stdout] ligne de sortie...`
- `#> [stderr] ligne d'erreur...`
- `>> {"status_code":0,...}` (inchangé)

Avantage : pas de nouveau préfixe, `>>` reste le JSON. Inconvénient : pollue le canal de commentaires (conçu pour le debug/aide, pas le contenu fonctionnel), les clients doivent parser le tag `[stdout]`/`[stderr]` dans le commentaire, et un client qui ignore les commentaires (comportement légitime par design ADR 0006 §9) perdrait toute la sortie.

### Option C — Chunks JSON intermédiaires

Garder le modèle `>>` pour tout, mais envoyer des objets JSON intermédiaires :

- `>> {"type":"chunk","stream":"stdout","data":"ligne 1\n"}`
- `>> {"type":"result","status_code":0,...}`

Avantage : un seul préfixe, tout est du JSON parseable. Inconvénient : overhead de sérialisation/parsing JSON par ligne, moins lisible pour l'humain, et le client doit distinguer les types dans le JSON (rupture sémantique du contrat « un `>>` = une réponse finale »).

---

## Décision

### Option A — Streaming avec trois préfixes distincts

**Décision du BO (Julien, 2026-03-18).**

### 1. Nouveaux préfixes de sortie

Le préfixe `>>` change de sémantique et deux nouveaux préfixes sont ajoutés :

| Préfixe | Rôle | Description |
|---------|------|-------------|
| `>> ` | **Sortie stdout** | Chaque ligne de stdout du processus enfant, envoyée immédiatement |
| `>>! ` | **Sortie stderr** | Chaque ligne de stderr du processus enfant, envoyée immédiatement |
| `>>> ` | **Réponse JSON finale** | Résultat structuré (5 champs) — anciennement `>> ` |

Les préfixes existants (`+>`, `#>`) restent inchangés. Le terminateur `.` reste inchangé.

**Choix du `!` pour stderr** : le `!` évoque visuellement l'alerte/attention, plus intuitif que `2` (numéro de file descriptor) pour un lecteur humain ou un LLM. Le TODO-030 mentionnait `>>2 ` comme alternative — le `!` est préféré pour la lisibilité et la cohérence avec les conventions de marquage d'erreur (ex: `!` en YAML, `!important` en CSS).

### 2. Détection des limites de ligne

- **Détection sur `\n`** : chaque occurrence de `\n` dans le flux stdout ou stderr du processus enfant déclenche l'envoi d'une ligne préfixée au client
- **Flush final** : si le processus enfant se termine avec des données restantes dans le buffer (pas de `\n` final), ces données sont envoyées comme dernière ligne
- **Pas de limite de taille par ligne** : les lignes très longues sont envoyées telles quelles — le transport SSH gère la fragmentation TCP

### 3. Flux de commande streamée

```
$ gitlab backup-full
.
>> [1/7] Nettoyage des anciens backups dans le conteneur...
>> [2/7] Création du backup GitLab (gitlab-backup create)...
>> 2026-03-18 13:39:18 UTC -- Dumping database ...
>>! zip warning: Permission denied for /var/opt/gitlab/tmp/cache
>> 2026-03-18 13:39:21 UTC -- [DONE]
>> [3/7] Recherche du fichier backup...
>> Fichier trouvé : /var/opt/gitlab/backups/1710765558_gitlab_backup.tar
>>> {"command":"gitlab backup-full","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

**Interaction avec le chaînage de commandes** : pour un bloc `cmd1 ; cmd2`, chaque commande exécutée produit ses propres lignes de streaming puis sa réponse `>>>` :

```
$ mastodon healthcheck ; gitlab backup-config
.
>>> {"command":"mastodon healthcheck","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
>> Configuration OK
>>> {"command":"gitlab backup-config","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

### 4. JSON final en mode streaming

Quand le contenu a été streamé, les champs `stdout` et `stderr` du JSON final sont **`null`** :

```json
{
  "command": "gitlab backup-full",
  "status_code": 0,
  "status_message": "executed",
  "stdout": null,
  "stderr": null
}
```

**Évolution de la sémantique `null` (addendum ADR 0003)** : l'ADR 0003 établit que `null` = « commande non exécutée » et `""` = « exécutée, rien produit ». Le streaming modifie cette convention :

| Valeur | Signification (ADR 0003) | Signification (cette ADR) |
|--------|--------------------------|---------------------------|
| `null` | Commande non exécutée (rejet) | Commande non exécutée **ou** contenu déjà streamé |
| `""` | Exécutée, rien produit | N'apparaît plus pour les commandes exécutées via processus enfant |

**C'est un compromis assumé** : le client perd la capacité de distinguer « rejeté » de « exécuté sans sortie » en regardant uniquement `stdout`/`stderr`. Le champ `status_message` reste le discriminant fiable : `"executed"` vs `"rejected: ..."` vs `"timeout..."`. En pratique, un client bien écrit utilise déjà `status_code` et `status_message` comme critères primaires, pas `stdout`/`stderr`.

### 5. Règle générale : chaque commande produit un `>>>`

**Tout traitement de commande — qu'il soit streamé ou non — se termine par exactement un `>>>` JSON.** C'est le signal universel de fin de réponse. Le client sait que le serveur a fini de répondre quand il reçoit la ligne `>>>`.

| Cas | Lignes `>>` / `>>!` avant | `>>>` final | `stdout`/`stderr` dans le JSON |
|-----|---------------------------|-------------|-------------------------------|
| **Commande exécutée** (processus enfant) | Oui (0 à N lignes) | `status_message: "executed"` | `null` (contenu streamé) |
| **Timeout** | Oui (lignes partielles) | `status_message: "timeout..."` | `null` |
| **Rejet** (inconnue, niveau, syntaxe) | Non | `status_message: "rejected: ..."` | `null` |
| **`help`** | Non (texte via `#>`) | `status_code: 0, status_message: "ok"` | `null` |
| **`list`** | Non | `status_code: 0, status_message: "ok"` | `stdout: Some(json)` |
| **`exit`** | Non | `status_code: 0, status_message: "ok"` | `null` |

**Changement pour `help`** : actuellement, `help` ne produit pas de réponse `>>` JSON — il envoie du texte humain via `#>` et c'est tout. Avec cette ADR, `help` doit aussi émettre un `>>>` final pour que le client sache que la réponse est terminée. C'est particulièrement important en mode session, où le client attend un `>>>` avant d'envoyer la commande suivante.

**`list` conserve `stdout: Some(...)`** : `list` n'est pas une commande externe — il produit du JSON en interne. Pas de streaming, le JSON de découverte est dans le champ `stdout` du `>>>` final. Le constructeur `Response::executed()` (avec `stdout: Some(...)`) reste nécessaire pour ce cas.

### 6. Protection contre le volume excessif (`max_stream_bytes`)

Le streaming supprime la troncature `max_stdout_chars` / `max_stderr_chars` pour les commandes exécutées. En remplacement, un compteur d'octets limite le volume total streamé :

```toml
[global]
max_stream_bytes = 10485760   # 10 Mo, défaut
```

Au-delà de `max_stream_bytes` octets streamés (stdout + stderr cumulés) :
1. Le streaming s'arrête (les lignes suivantes sont silencieusement ignorées)
2. Un avertissement unique est émis : `>>! ssh-frontiere: output truncated (max 10MB exceeded)`
3. L'exécution continue normalement (le processus enfant n'est pas kill)
4. Le `>>>` final reflète le code de sortie réel du processus

Les constantes `max_stdout_chars` / `max_stderr_chars` restent dans la config pour ne pas casser le chargement, mais sont ignorées pour les commandes exécutées. Elles pourraient être supprimées dans une phase ultérieure.

### 7. Pas d'option streaming on/off

Le streaming est le comportement **par défaut et unique**. Pas de `streaming = false` ni de fallback :

- Le mode bufferisé n'a aucun avantage fonctionnel — le streaming est strictement supérieur (feedback temps réel, empreinte mémoire réduite)
- Aucun client de production n'existe — les seuls consommateurs sont les tests E2E et les scripts de déploiement internes
- La complexité d'un double chemin de code (bufferisé + streaming) n'est pas justifiée

### 8. Impact sur le code

#### `protocol.rs` — Préfixes

La fonction `write_response` change de préfixe, et deux nouvelles fonctions sont ajoutées :

```rust
// Avant
pub fn write_response(writer: &mut impl Write, json: &str) -> Result<(), String> {
    writeln!(writer, ">> {json}")...
}

// Après
pub fn write_response(writer: &mut impl Write, json: &str) -> Result<(), String> {
    writeln!(writer, ">>> {json}")...
}

// Nouvelles fonctions
pub fn write_stdout_line(writer: &mut impl Write, line: &str) -> Result<(), String> {
    writeln!(writer, ">> {line}")...
}

pub fn write_stderr_line(writer: &mut impl Write, line: &str) -> Result<(), String> {
    writeln!(writer, ">>! {line}")...
}
```

**Flush par ligne** : `write_stdout_line` et `write_stderr_line` font un `flush()` après chaque `writeln!` pour garantir l'envoi immédiat au client. Cela signifie un syscall `write` par ligne — le `BufWriter` de l'orchestrateur n'amortit pas dans ce cas. C'est le bon comportement pour du streaming temps réel : la latence prime sur le throughput. Pour les commandes qui produisent des milliers de lignes, la protection `max_stream_bytes` (§6) limite le volume total.

#### `dispatch.rs` — `execute_command()`

La signature change : le writer est passé en paramètre, et le retour utilise un enum typé au lieu de `Result<(i32, String, String), String>` :

```rust
/// Résultat de l'exécution d'une commande streamée
pub enum ExecuteResult {
    /// Processus terminé normalement (code de sortie)
    Exited(i32),
    /// Processus tué par signal (numéro de signal)
    Signaled(i32),
    /// Timeout dépassé
    Timeout,
    /// Erreur au spawn
    SpawnError(String),
}
```

Cet enum remplace le pattern fragile `Err(e) if e.contains("timeout")` du code actuel. Le `status.code().unwrap_or(1)` actuel (dispatch.rs:287) est également corrigé : quand un processus est tué par signal, `code()` retourne `None` — l'enum `Signaled(signal)` permet de produire un code conforme aux conventions Unix (128 + signal).

**Lecture concurrente stdout/stderr** — Deux threads avec channel commun :

```
Thread spawn (stderr) ──→ mpsc::Sender<StreamLine>
                                                     ├──→ Thread main : consomme le Receiver
Thread spawn (stdout) ──→ mpsc::Sender<StreamLine>       et écrit >> / >>! sur le writer
```

```rust
enum StreamLine {
    Stdout(String),
    Stderr(String),
}
```

- Deux threads `std::thread::spawn` (stdlib, zéro dépendance) : un pour stdout, un pour stderr
- Chaque thread lit son pipe via `BufReader::read_line()` et envoie les lignes sur un `mpsc::Sender<StreamLine>` partagé
- Le thread principal consomme le `Receiver`, écrit `>> ` ou `>>! ` selon le type, et gère le timeout
- L'ordre des lignes reflète l'ordre d'arrivée réel (pas d'ordonnancement artificiel)

La fonction `read_pipe()` existante est supprimée (plus de lecture en bloc).

#### `chain.rs` — `execute_single_command()`

Le flux d'exécution change. Le constructeur `Response::executed()` reste pour les built-in (`list`), et un nouveau `Response::streamed()` est ajouté pour les commandes exécutées via processus enfant :

```rust
// Avant
match execute_command(&cmd_refs, timeout, session_id) {
    Ok((code, stdout, stderr)) => {
        let stdout = truncate_output(&stdout, config.global.max_stdout_chars);
        let stderr = truncate_output(&stderr, config.global.max_stderr_chars);
        let resp = Response::executed(raw_command, code, stdout, stderr);
        let _ = write_response(writer, &resp.to_json());
    }
}

// Après
match execute_command(&cmd_refs, timeout, session_id, writer) {
    ExecuteResult::Exited(code) => {
        let resp = Response::streamed(raw_command, code);
        let _ = write_response(writer, &resp.to_json());
    }
    ExecuteResult::Signaled(signal) => {
        let resp = Response::streamed(raw_command, 128 + signal);
        let _ = write_response(writer, &resp.to_json());
    }
    ExecuteResult::Timeout => {
        let resp = Response::timeout(&desc, timeout);
        let _ = write_response(writer, &resp.to_json());
    }
    ExecuteResult::SpawnError(e) => {
        let resp = Response::rejected(raw_command, &format!("execution error: {e}"), EXIT_REJECTED);
        let _ = write_response(writer, &resp.to_json());
    }
}
```

#### `output.rs` — `Response`

Nouveau constructeur pour les commandes streamées :

```rust
impl Response {
    /// Commande exécutée avec streaming (stdout/stderr déjà envoyés via >> / >>!)
    pub fn streamed(command: &str, code: i32) -> Self {
        Response {
            command: command.to_string(),
            status_code: code,
            status_message: "executed".to_string(),
            stdout: None,
            stderr: None,
        }
    }
}
```

Les constructeurs existants restent :
- `Response::executed()` (avec `stdout: Some(...)`) : nécessaire pour `list`
- `Response::rejected()` : inchangé
- `Response::timeout()` : inchangé

#### `chain.rs` — `write_help_text()`

Le texte d'aide protocole doit refléter les nouveaux préfixes :

```rust
// Avant
let _ = write_comment(writer, "  >>    Reponse JSON serveur");

// Après
let _ = write_comment(writer, "  >>    Sortie stdout (streaming)");
let _ = write_comment(writer, "  >>!   Sortie stderr (streaming)");
let _ = write_comment(writer, "  >>>   Reponse JSON finale");
```

De plus, `write_help_text()` (et le raccourci `help` dans l'orchestrateur) doit émettre un `>>>` final après les lignes `#>` :

```rust
// Après les lignes #> d'aide
let resp = Response {
    command: "help".to_string(),
    status_code: 0,
    status_message: "ok".to_string(),
    stdout: None,
    stderr: None,
};
let _ = write_response(writer, &resp.to_json());
```

### 9. Séquence de shutdown des threads (thread safety)

Le `writer` (stdout du serveur) est utilisé par le thread principal uniquement. Les threads de lecture envoient les lignes via le channel. Cela garantit l'absence de data race.

**Séquence de fin d'exécution** (après que `try_wait()` retourne `Some(status)`) :

```
1. try_wait() retourne Some(status)
2. join() des deux threads de lecture
   → les threads se terminent car les pipes sont fermés (EOF sur read_line)
   → les dernières lignes sont envoyées sur le channel avant la mort du thread
3. Drainer le channel : recv() en boucle jusqu'à Err (channel déconnecté)
   → écrire les lignes restantes sur le writer
4. Produire le ExecuteResult
```

Le `join()` est essentiel : sans lui, des lignes pourraient être en transit dans le channel au moment où le thread principal conclut. Le drain après `join()` garantit que toutes les lignes sont écrites avant le `>>>` final.

**Code de sortie par signal** : quand un processus est tué par signal (SIGTERM, SIGKILL), `std::process::ExitStatus::code()` retourne `None`. L'implémentation doit utiliser `status.signal()` (via `std::os::unix::process::ExitStatusExt`) pour produire `ExecuteResult::Signaled(signal)`.

**Séquence de timeout** :

```
1. Timeout global dépassé
2. send_signal_to_group(SIGTERM)
3. Drainer les lignes restantes du channel (délai court)
4. Émettre >>! ssh-frontiere: command timed out
5. send_signal_to_group(SIGKILL) si toujours vivant
6. join() des threads
7. Drainer le channel final
8. Produire ExecuteResult::Timeout
```

**Processus enfant qui fork** : si le processus enfant fait `fork()`, le sous-processus hérite du pipe. Le thread de lecture ne verra pas EOF tant que le sous-processus est vivant. Le kill du process group (`kill -- -$PID`) tue aussi les sous-processus, ce qui ferme le pipe. Le mécanisme existant dans `kill_process()` (SIGTERM → délai → SIGKILL au process group) gère ce cas.

### 10. Rétrocompatibilité

**Rupture intentionnelle** sur le préfixe de réponse JSON :

| Avant (implémentation actuelle) | Après (cette ADR) |
|---------------------------------|-------------------|
| `>> {json}` | `>>> {json}` |

**Impact client** :

- Les clients qui parsent `>> ` pour extraire le JSON final devront migrer vers `>>> `
- Aucun client de production n'existe — les seuls consommateurs sont les tests E2E Docker et les scripts de déploiement internes
- La migration est mécanique : remplacer le parsing de `>> ` par `>>> ` dans les scripts bash (grep/sed)

**Impact tests** :

| Suite de tests | Fichier(s) | Impact |
|----------------|-----------|--------|
| Tests unitaires protocol.rs | `src/protocol_tests.rs` | `write_response` → vérifier `>>>` au lieu de `>>` |
| Tests d'intégration | `tests/integration.rs` | Adapter le helper `run_protocol_full` (prendre `>>>` au lieu de `>>` pour le JSON) |
| Tests de conformité ADR 0003 | `tests/conformance.rs` | Adapter le parsing des réponses |
| Tests E2E SSH | `tests/e2e-ssh/scenarios/*.sh` (~67 fichiers) | Remplacer `>> ` par `>>> ` dans les grep/assertions |

Pour les tests d'intégration, le helper qui extrait le JSON devra ignorer les lignes `>> ` (stdout streamé) et prendre la première ligne `>>> ` comme réponse JSON. Ce changement est localisé dans le helper.

### 11. Addendum à l'ADR 0006

Cette ADR amende le préfixe de réponse de l'ADR 0006. L'ADR 0006 §1 définit abstraitement le préfixe `>` pour « Répond ». L'implémentation utilise `>>` (double chevron, convention de direction serveur). Cette ADR remplace ce préfixe unique par une famille de trois :

| Préfixe (implémentation) | Rôle | Direction | Description |
|--------------------------|------|-----------|-------------|
| `>>` | Sortie stdout | Serveur → client | Ligne de stdout du processus enfant (streaming) |
| `>>!` | Sortie stderr | Serveur → client | Ligne de stderr du processus enfant (streaming) |
| `>>>` | Réponse finale | Serveur → client | Réponse JSON structurée (format 5 champs) |

### 12. Addendum à l'ADR 0003

Le contrat JSON (ADR 0003) évolue sur deux points :

1. **Champ `command`** : le format passe de 4 à 5 champs avec l'ajout de `command` (déjà implémenté depuis l'alignement 003, Phase 3). Les 5 champs obligatoires sont : `command`, `status_code`, `status_message`, `stdout`, `stderr`.

2. **Sémantique de `null`** : `null` dans `stdout`/`stderr` signifie désormais « pas de contenu dans ce champ » — soit parce que la commande n'a pas été exécutée (rejet), soit parce que le contenu a été streamé via `>>` / `>>!`. Le discriminant fiable reste `status_message` et `status_code`.

---

## Conséquences

### Positives

- **Retour en temps réel** : les workflows Forgejo Actions affichent les logs pendant l'exécution — les opérateurs voient la progression
- **Diagnostic facilité** : en cas d'échec au milieu d'un script, les dernières lignes streamées indiquent l'étape en cours
- **Distinction stdout/stderr en temps réel** : le client peut séparer les flux grâce aux préfixes `>>` et `>>!`
- **Empreinte mémoire réduite** : plus besoin de stocker toute la sortie en RAM avant de l'envoyer — chaque ligne est écrite et oubliée
- **JSON final allégé** : plus de sérialisation/échappement de gros blocs de texte dans le JSON
- **Zéro nouvelle dépendance** : `std::thread` et `mpsc::channel` sont dans la stdlib Rust
- **Protocole uniforme** : chaque commande (y compris `help`) se termine par un `>>>`, simplifiant la logique client

### Négatives

- **Rupture du contrat `>>` → `>>>`** : tous les clients et tests qui parsent `>>` pour le JSON doivent être mis à jour (~67 scénarios E2E + tests d'intégration)
- **Complexité d'implémentation** : la lecture concurrente stdout/stderr nécessite deux threads, un channel, et une séquence de shutdown (join + drain) — plus complexe que le `read_to_string` actuel
- **Perte de la distinction `null` vs `""`** : le client ne peut plus distinguer « rejeté » de « exécuté sans sortie » via `stdout` seul — il doit utiliser `status_message`
- **Interleaving stdout/stderr** : les lignes des deux flux s'entrelacent dans l'ordre d'arrivée — le client qui veut reconstruire stdout seul doit filtrer les `>>!`
- **Un syscall par ligne** : le flush après chaque `writeln!` produit un appel système par ligne streamée — overhead acceptable pour le streaming temps réel, mais sensible pour les commandes très verbeuses (atténué par `max_stream_bytes`)

### Risques

- **Client naïf perturbé** : un client qui lit tout comme du texte brut sans parser les préfixes verra des lignes `>>!` mélangées. Atténuation : les préfixes sont explicites et documentés
- **Flush partiel en cas de crash** : si le processus enfant est kill (SIGKILL), les données dans le buffer du pipe noyau pourraient être perdues. Atténuation : `join()` des threads + drain du channel après `wait()` capturent les données restantes
- **Deadlock par fork du processus enfant** : si le processus enfant fork un sous-processus qui hérite du pipe, le thread de lecture ne voit pas EOF. Atténuation : kill du process group (SIGTERM → SIGKILL) ferme les pipes en tuant tous les processus du groupe
- **Volume excessif** : une commande produisant des Go de sortie saturerait le canal SSH. Atténuation : `max_stream_bytes` (§6) coupe le streaming au-delà du seuil configuré (défaut 10 Mo)
- **Lignes très longues** : une commande produisant une seule ligne de 100 Mo (ex: dump base64 non découpé) sera envoyée d'un bloc. Atténuation : `max_stream_bytes` s'applique au volume total, pas par ligne — si les 100 Mo dépassent le seuil, la ligne est tronquée

---

## Plan de migration

### Étape 1 : Protocole (`protocol.rs`)

- Changer `write_response()` : `>> ` → `>>> `
- Ajouter `write_stdout_line()` et `write_stderr_line()`
- Adapter les tests unitaires `protocol_tests.rs`

### Étape 2 : Exécution streaming (`dispatch.rs`)

- Ajouter l'enum `ExecuteResult`
- Modifier `execute_command()` : nouvelle signature avec `writer`, lecture via threads + channel, séquence join + drain
- Corriger la gestion du code de sortie par signal (`Signaled`)
- Supprimer `read_pipe()`
- Ajouter `max_stream_bytes` dans `config.rs` (section `[global]`)
- Adapter les tests unitaires `dispatch_tests.rs`

### Étape 3 : Intégration (`chain.rs`, `output.rs`)

- Adapter `execute_single_command()` pour le nouveau flux avec `ExecuteResult`
- Ajouter `Response::streamed()`
- Mettre à jour `write_help_text()` : nouveaux préfixes dans l'aide + émission d'un `>>>` final
- Mettre à jour l'orchestrateur : `help` sans préfixe → émettre un `>>>` final aussi

### Étape 4 : Tests d'intégration et conformité

- Adapter `tests/integration.rs` : modifier le helper de parsing (`>>>` au lieu de `>>`)
- Adapter `tests/conformance.rs` : idem
- Ajouter des scénarios vérifiant le streaming (présence de lignes `>>` avant `>>>`)

### Étape 5 : Tests E2E SSH

- Mettre à jour les ~67 scénarios `tests/e2e-ssh/scenarios/*.sh` : remplacer le parsing `>> ` par `>>> `
- Ajouter des scénarios de streaming (commande longue avec sortie progressive)
- Vérifier l'interleaving stdout/stderr

---

## Tests nécessaires

### Unitaires (`protocol.rs`)

1. `write_response` écrit `>>> {json}\n` (plus `>> `)
2. `write_stdout_line` écrit `>> {line}\n`
3. `write_stderr_line` écrit `>>! {line}\n`

### Unitaires (`dispatch.rs`)

4. `execute_command` avec commande produisant stdout → lignes `>> ` écrites sur le writer
5. `execute_command` avec commande produisant stderr → lignes `>>! ` écrites sur le writer
6. `execute_command` avec stdout + stderr → lignes correctement préfixées
7. Timeout : lignes partiellement streamées, puis `ExecuteResult::Timeout`
8. Commande sans sortie → aucune ligne `>>` / `>>!`, `ExecuteResult::Exited(0)`
9. Processus tué par signal → `ExecuteResult::Signaled(signal)`
10. Volume dépassant `max_stream_bytes` → avertissement `>>!` émis, streaming arrêté
11. Ligne très longue (sans `\n`) → envoyée d'un bloc si sous `max_stream_bytes`

### Unitaires (`output.rs`)

12. `Response::streamed()` → `stdout: null, stderr: null, status_message: "executed"`
13. `Response::streamed()` avec code non-zéro → `status_code` correct

### Intégration

14. Commande avec sortie multi-lignes → toutes les lignes préfixées `>>` reçues, suivies de `>>>`
15. Commande rejetée → uniquement `>>>` (pas de lignes `>>` / `>>!`)
16. Mode session : streaming correct par commande, chaque commande a son `>>>` final
17. Chaînage de commandes (`; & |`) : chaque commande streamée a ses propres lignes `>>` + `>>>`
18. `help` → lignes `#>` suivies d'un `>>>` final

### E2E SSH

19. Scénario streaming stdout (commande longue) → lignes `>>` visibles
20. Scénario stderr → lignes `>>!` reçues
21. Parsing `>>>` pour le JSON final → tous les scénarios existants adaptés
22. Scénario mixte stdout + stderr → les deux types de préfixes présents
23. Scénario volume excessif → avertissement de troncature

---

## Attribution

- **Julien (BO)** : décision du streaming ligne par ligne, choix des préfixes (`>> ` stdout, `>>> ` JSON final), détection sur `\n`, champs `stdout`/`stderr` à `null` dans le JSON streamé, pas d'option de configuration (streaming = comportement unique), pas de rétrocompatibilité
- **Claude (PM/Tech Lead)** : préfixe `>>! ` pour stderr (choix du `!` pour lisibilité), analyse de la lecture concurrente (deux threads stdlib + `mpsc::channel` commun + séquence join/drain), enum `ExecuteResult` pour typage fort du retour, `max_stream_bytes` pour protection anti-DoS, `help` émet un `>>>` final pour uniformité protocole, gestion du code de sortie par signal, impact sur le code (4 fichiers : dispatch.rs, protocol.rs, chain.rs, output.rs), plan de migration en 5 étapes, addendums ADR 0003 et ADR 0006
- **Agents Claude Code** : implémentation, tests
