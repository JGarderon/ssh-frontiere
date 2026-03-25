+++
title = "設定"
description = "SSH-Frontièreのconfig.tomlファイルを作成する"
date = 2026-03-24
weight = 3
+++

# 設定

SSH-Frontièreは、ドメイン、アクション、アクセスレベル、引数、認証トークンを宣言するためにTOMLファイルを使用します。

## ファイルの場所

**デフォルトパス**：`/etc/ssh-frontiere/config.toml`

**上書き**（優先順位順）：
1. `authorized_keys`の`command=`行での`--config <path>`
2. 環境変数`SSH_FRONTIERE_CONFIG`
3. デフォルトパス

**推奨パーミッション**：`root:forge-runner 640`（サービスアカウントに合わせてグループを調整してください）。

## ファイル構造

```toml
[global]                              # 全般設定
[domains.<id>]                        # 機能ドメイン
  [domains.<id>.actions.<id>]         # 許可されたアクション
    [domains.<id>.actions.<id>.args]  # 名前付き引数（任意）
[auth]                                # RBAC認証（任意）
  [auth.tokens.<id>]                  # シークレット、レベル、タグを持つトークン
```

## `[global]`セクション

| キー | 型 | デフォルト | 説明 |
|------|------|-----------|------|
| `log_file` | string | **必須** | JSONログファイルのパス |
| `default_timeout` | integer | `300` | デフォルトのタイムアウト（秒） |
| `max_stdout_chars` | integer | `65536` | stdout制限（64 KB） |
| `max_stderr_chars` | integer | `16384` | stderr制限（16 KB） |
| `max_output_chars` | integer | `131072` | グローバルハード制限（128 KB） |
| `max_stream_bytes` | integer | `10485760` | ストリーミング量制限（10 MB） |
| `timeout_session` | integer | `3600` | セッションkeepaliveタイムアウト |
| `max_auth_failures` | integer | `3` | ロックアウトまでの認証試行回数 |
| `ban_command` | string | `""` | IPバンコマンド（プレースホルダ`{ip}`） |
| `log_comments` | bool | `false` | クライアントの`#`行をログに記録 |
| `expose_session_id` | bool | `false` | バナーにセッションUUIDを表示 |

`log_level`、`default_level`、`mask_sensitive`キーは、古い設定との後方互換性のためにパーサーで受け入れられますが、現在は使用されていません。

## `[domains]`セクション

**ドメイン**は機能的なスコープです（例：`forgejo`、`infra`、`notify`）。各ドメインには許可された**アクション**が含まれます。

```toml
[domains.forgejo]
description = "Git forge infrastructure"

[domains.forgejo.actions.backup-config]
description = "Backup the configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
max_body_size = 65536       # ボディ制限（64 KB、任意）
```

各アクションは以下のキーを受け付けます：`description`（必須）、`level`（必須）、`execute`（必須）、`timeout`（任意、グローバルを上書き）、`tags`（任意）、`max_body_size`（任意、デフォルト65536バイト — `+body`プロトコル用に制限）。

### 信頼レベル

厳密な階層：`read` < `ops` < `admin`

| レベル | 用途 |
|--------|------|
| `read` | 読み取り専用：healthcheck、status、list |
| `ops` | 定常運用：backup、deploy、restart |
| `admin` | すべてのアクション＋管理 |

### 引数

引数はTOML辞書として宣言します：

```toml
[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }
```

| フィールド | 型 | 説明 |
|------------|------|------|
| `type` | string | `"enum"`または`"string"` |
| `values` | list | 許可された値（`enum`用） |
| `default` | string | デフォルト値（引数を任意にする） |
| `sensitive` | bool | `true`の場合、ログでマスキング |
| `free` | bool | `true`の場合、制約なしで任意の値を受け入れ |

### `execute`内のプレースホルダ

| プレースホルダ | 説明 |
|----------------|------|
| `{domain}` | ドメイン名（常に利用可能） |
| `{arg_name}` | 対応する引数の値 |

### 可視性タグ

タグはアクションへのアクセスを水平的にフィルタリングします。タグのないアクションはすべてのユーザーがアクセスできます。タグのあるアクションは、少なくとも1つのタグを共有するアイデンティティのみがアクセスできます。

```toml
[domains.forgejo.actions.deploy]
# ...
tags = ["forgejo", "deploy"]
```

## `[auth]`セクション（任意）

RBAC認証は、チャレンジ・レスポンスによる権限昇格を可能にします：

```toml
[auth]
challenge_nonce = false              # true = アンチリプレイノンスモード

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="   # Base64エンコードされたシークレット
level = "ops"                               # 付与されるレベル
tags = ["forgejo"]                          # 可視性タグ
```

シークレットには`b64:`プレフィックスを付け、Base64でエンコードする必要があります。シークレットの生成方法：

```bash
echo -n "my-random-secret" | base64
# bXktcmFuZG9tLXNlY3JldA==
```

## 読み込み時の検証

設定は各読み込み時に完全に検証されます（フェイルファスト）。エラー時にはプログラムがコード129で終了します。検証内容：

- 正しいTOML構文
- 少なくとも1つのドメイン、ドメインごとに少なくとも1つのアクション
- 各アクションに有効な`execute`と`level`がある
- `execute`内のプレースホルダ`{arg}`が宣言された引数と一致
- enum引数に少なくとも1つの許可された値がある
- デフォルト値が許可された値のリストに含まれている
- `max_stdout_chars`と`max_stderr_chars` <= `max_output_chars`

## 完全な例

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 300
max_stdout_chars = 65536
max_stderr_chars = 16384
max_output_chars = 131072
timeout_session = 3600
max_auth_failures = 3

[domains.forgejo]
description = "Git forge infrastructure"

[domains.forgejo.actions.backup-config]
description = "Backup the Forgejo configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"

[domains.forgejo.actions.deploy]
description = "Deployment with version tag"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.infra]
description = "Server infrastructure"

[domains.infra.actions.healthcheck]
description = "Service health check"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[auth]
challenge_nonce = false

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

すべてのユースケースを網羅した詳細なガイドについては、リポジトリ内の[完全な設定ガイド](https://github.com/nothus-forge/ssh-frontiere/blob/main/docs/references/configuration.md)を参照してください。

---

**次へ**：[デプロイ](@/installation/deploiement.md) — 本番環境へのデプロイ。
