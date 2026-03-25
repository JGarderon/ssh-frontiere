# SSH Frontière v3.0.0

**Website**: [pages.nothus.fr/ssh-frontiere](https://pages.nothus.fr/ssh-frontiere/ja/)

[![License: EUPL-1.2](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.md)

Rust 製の制限付き SSH ログインシェル — サーバーへのすべての SSH 接続に対する、単一の安全なエントリーポイントです。

SSH Frontière は `/etc/passwd` のデフォルトシェル（`/bin/bash`）を置き換え、**セキュアなディスパッチャー**として機能します。すべての SSH コマンドを TOML ホワイトリストで検証し、3 段階の RBAC アクセス制御を適用し、結果を stdin/stdout 上のヘッダーベースのプロトコルを通じて構造化 JSON として返します。

## 目的

SSH Frontière は SSH サービスアカウント向けの**セキュリティコンポーネント**です。

- **CI/CD ランナー**（Forgejo Actions、GitHub Actions）：コンテナからのインフラ操作
- **AI エージェント**（Claude Code など）：信頼レベルを伴うサーバーへの制御アクセス
- **自動メンテナンス**：バックアップ、デプロイ、ヘルスチェック

このプログラムは**同期的かつワンショット**です。SSH は接続ごとに新しいプロセスを生成し、ディスパッチャーが検証・実行後に終了します。デーモンなし、非同期なし、Tokio なし。

## インストール

### 前提条件

- Rust 1.70 以上、`x86_64-unknown-linux-musl` ターゲットを含む
- `make`（省略可、ショートカット用）

### コンパイル

```bash
# make 経由
make release

# または直接
cargo build --release --target x86_64-unknown-linux-musl
```

生成された静的バイナリ（`target/x86_64-unknown-linux-musl/release/ssh-frontiere`、約 1〜2 MB）はシステム依存なしでデプロイできます。

```bash
sudo cp target/x86_64-unknown-linux-musl/release/ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere
```

## TOML 設定

デフォルトファイル：`/etc/ssh-frontiere/config.toml`。
上書き：`--config <パス>` または環境変数 `SSH_FRONTIERE_CONFIG`。

### 完全な例

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300          # デフォルトタイムアウト（秒）
default_level = "read"         # デフォルト RBAC レベル
mask_sensitive = true           # ログ内の機密引数をマスク
max_stdout_chars = 65536       # キャプチャした stdout の上限
max_stderr_chars = 16384       # キャプチャした stderr の上限
max_output_chars = 131072      # グローバルハード上限
timeout_session = 3600         # セッションキープアライブのタイムアウト（秒）
max_auth_failures = 3          # ロックアウトまでの認証試行回数
log_comments = false           # クライアントのコメントをログに記録
ban_command = ""               # IP バンコマンド（例："/usr/sbin/iptables -A INPUT -s {ip} -j DROP"）

# --- RBAC 認証（省略可）---

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # b64: プレフィックス付き Base64 エンコードシークレット
level = "ops"                                # このトークンで付与されるレベル

[auth.tokens.admin-deploy]
secret = "b64:c2VjcmV0LWFkbWluLWRlcGxveQ=="
level = "admin"

# --- ドメインとアクション ---

[domains.forgejo]
description = "Git フォージインフラ"

[domains.forgejo.actions.backup-config]
description = "Forgejo 設定のバックアップ"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
args = []

[domains.forgejo.actions.deploy]
description = "バージョンのデプロイ"
level = "admin"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {version}"

[[domains.forgejo.actions.deploy.args]]
name = "version"
type = "enum"
values = ["latest", "stable", "canary"]

[domains.infra]
description = "サーバーインフラ"

[domains.infra.actions.healthcheck]
description = "ヘルスチェック"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"
args = []

[domains.infra.actions.set-password]
description = "サービスパスワードの変更"
level = "admin"
timeout = 30
execute = "sudo /usr/local/bin/set-password.sh {password}"

[[domains.infra.actions.set-password.args]]
name = "password"
type = "string"
sensitive = true    # mask_sensitive = true のときログでマスクされる
```

### 引数の型

| 型 | 説明 | バリデーション |
|----|------|--------------|
| `string` | 自由テキスト | 最大 256 文字 |
| `enum` | リストからの値 | `values` 内のいずれかに一致する必要あり |

### `execute` 内のプレースホルダー

- `{domain}`：ドメイン名に置き換えられる（常に利用可能）
- `{arg_name}`：対応する引数の値に置き換えられる

## デプロイ

### 1. ログインシェル（`/etc/passwd`）

```bash
# サービスアカウントの作成
sudo useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

プログラムは `sshd` によりログインシェルとして直接呼び出されます。

### 2. `authorized_keys` での SSH 鍵設定

```
# ~forge-runner/.ssh/authorized_keys

# CI ランナー鍵（ops レベル）
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-key

# モニタリング鍵（読み取り専用レベル）
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitor-key

# 管理者鍵（admin レベル）
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

`command=` オプションは、クライアントが送信したコマンドに関わらず、選択した `--level` で ssh-frontiere の実行を強制します。`restrict` オプションはポートフォワーディング、エージェントフォワーディング、PTY、X11 を無効化します。

### 3. sudoers（第 3 層）

```
# /etc/sudoers.d/ssh-frontiere
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/set-password.sh
```

TOML ホワイトリストに記載され、**かつ** sudoers で承認されたコマンドのみが昇格権限で実行できます。

## ヘッダープロトコル

SSH Frontière は stdin/stdout 上で 4 つのプレフィックスを持つテキストプロトコルを使用します（ADR 0006）。

### プレフィックス

| プレフィックス | 役割 | 方向 |
|-------------|------|------|
| `+` | **設定**：ディレクティブ（`capabilities`、`challenge`、`auth`、`session`） | 双方向 |
| `#` | **コメント**：情報、バナー、メッセージ | 双方向 |
| `$` | **コマンド**：実行するコマンド | クライアント → サーバー |
| `>` | **応答**：JSON レスポンス | サーバー → クライアント |

### 接続フロー

```
クライアント                         サーバー
  |                                    |
  |  <-- バナー + capabilities -----  |   # ssh-frontiere 3.0.0
  |  <-- チャレンジ nonce ----------  |   + capabilities rbac, session, help
  |                                    |   + challenge nonce=a1b2c3...
  |                                    |   # type "help" for available commands
  |                                    |
  |  --- +auth（省略可）---------->   |   + auth token=runner-ci proof=deadbeef...
  |  --- +session（省略可）------->   |   + session keepalive
  |  --- # コメント（省略可）------>  |   # client-id: forgejo-runner-12
  |  --- 空行 -------------------->   |   （ヘッダー終了）
  |                                    |
  |  --- ドメイン アクション [引数] ->  |   forgejo backup-config
  |  --- . ------------------------>  |   . （コマンドブロック終了）
  |  <-- >> stdout（ストリーミング）-  |   >> Backup completed
  |  <-- >>> JSON レスポンス ------   |   >>> {"command":"forgejo backup-config","status_code":0,...}
  |                                    |
```

### JSON レスポンス（4 フィールド）

各コマンドは JSON オブジェクトを含む `>>>` レスポンスを生成します。

```json
{
  "command": "forgejo backup-config",
  "status_code": 0,
  "status_message": "executed",
  "stdout": null,
  "stderr": null
}
```

- `stdout`/`stderr` = `null`：出力は `>>` / `>>!` プレフィックス経由でストリーミングされた
- `status_code` = 0：成功（パススルーでの子プロセス終了コード）

### 終了コード

| コード | 意味 |
|--------|------|
| 0 | 成功 |
| 1-127 | 子コマンドの終了コード（パススルー） |
| 128 | コマンド拒否 |
| 129 | 設定エラー |
| 130 | タイムアウト |
| 131 | RBAC レベル不足 |
| 132 | プロトコルエラー |
| 133 | body stdin が途中でクローズ |

## 具体的な使用例

### ワンショットモード

```bash
# シンプルなパイプ：
{
  echo "infra healthcheck"
  echo "."
} | ssh forge-runner@server
```

### セッションモード（キープアライブ）

セッションモードでは、1 つの SSH 接続で複数のコマンドを送信できます。

```bash
{
  echo "+ session keepalive"
  echo "infra healthcheck"
  echo "."
  echo "forgejo backup-config"
  echo "."
  echo "exit"
  echo "."
} | ssh forge-runner@server
```

サーバーは各コマンドに対して `>>>` JSON 行を返します。

### RBAC 認証（レベル昇格）

`--level=read` のクライアントは、チャレンジ・レスポンスで `ops` または `admin` に昇格できます。

```bash
{
  echo "+ auth token=runner-ci proof=<sha256-hex>"
  echo "forgejo backup-config"    # ops が必要、トークンで承認済み
  echo "."
} | ssh forge-runner@server
```

`proof` は `challenge_nonce = false` のとき `SHA-256(secret)`、`challenge_nonce = true` のとき `SHA-256(XOR(secret || nonce, secret))` です。有効レベルは `max(--level, token.level)` となります。

### ディスカバリー（help / list）

```bash
# アクセス可能なコマンドの全リスト
{ echo "help"; echo "."; } | ssh forge-runner@server

# ドメインの詳細
{ echo "help forgejo"; echo "."; } | ssh forge-runner@server

# 短縮リスト（ドメイン + アクション + 説明、JSON）
{ echo "list"; echo "."; } | ssh forge-runner@server
```

`help` および `list` コマンドは、クライアントの有効レベルでアクセス可能なアクションのみ表示します。

## セキュリティ

### 3 層の多層防御

| 層 | メカニズム | 保護内容 |
|----|-----------|---------|
| 1 | `authorized_keys` の `command=` + `restrict` | `--level` を強制、フォワーディング/PTY をブロック |
| 2 | `ssh-frontiere`（ログインシェル） | TOML ホワイトリストに対してコマンドを検証 |
| 3 | sudoers の `sudo` ホワイトリスト | 特権システムコマンドを制限 |

攻撃者が第 1 層を突破した場合（鍵の侵害）でも、第 2 層がホワイトリスト外のすべてのコマンドをブロックします。第 3 層はシステム権限を制限します。

### 文法パーサー、ブラックリストではない

**ssh-frontiere はシェルではありません。** セキュリティは文字フィルタリングではなく、**文法パーサー**に基づいています。

- 想定される文法は `domain action [args]` — この構造に一致しないものはすべて拒否されます
- 引用符内の特殊文字（`|`、`;`、`&`、`$` など）は引数の**内容**であり、シェル構文ではありません — 有効です
- 「禁止文字」は存在しません — 文法があり、それに適合しないものが拒否されます
- `std::process::Command` はシェルを介さず直接実行します — インジェクションは構造的に不可能です

### このプログラムが絶対にしないこと

- シェルの呼び出し（`/bin/bash`、`/bin/sh`）
- パイプ、リダイレクト、連鎖の受け入れ（`|`、`>`、`&&`、`;`）
- ホワイトリストに記載されていないコマンドの実行
- インタラクティブ TTY へのアクセス提供

### 追加の保護機能

- コマンドごとの**タイムアウト**（SIGTERM 後 SIGKILL でプロセスグループをキル）
- N 回の認証失敗後の**ロックアウト**（設定可能、デフォルト：3 回）
- 設定可能な外部コマンドによるオプションの **IP バン**（`ban_command`）
- JSON ログ内の機密引数の**マスキング**
- キャプチャした出力（stdout、stderr）の**サイズ制限**
- セッション認証成功後に再生成される**アンチリプレイ nonce**
- 子プロセスへの **env_clear()**（`PATH` のみ保持）

## テスト

```bash
# ユニットテストおよびインテグレーションテスト
make test

# エンド・ツー・エンド SSH テスト（Docker 必須）
make e2e

# リント（fmt + clippy）
make lint

# 依存関係のセキュリティ監査
make audit
```

E2E テスト（`make e2e`）は SSH サーバーとクライアントを含む Docker Compose 環境を起動し、プロトコル（PRO-*）、認証（AUT-*）、セッション（SES-*）、セキュリティ（SEC-*）、堅牢性（ROB-*）、ロギング（LOG-*）を網羅するシナリオを実行します。

## コントリビューション

コントリビューションを歓迎します！詳細は[コントリビューションガイド](CONTRIBUTING.md)をご覧ください。

## ライセンス

このプロジェクトは[欧州連合公衆ライセンス（EUPL-1.2）](LICENSE.md)の下で配布されています。

Copyright (c) Julien Garderon, 2024-2026
