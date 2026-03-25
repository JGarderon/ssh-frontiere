+++
title = "ドメインとアクション"
description = "SSH-Frontièreでドメインとアクションを設定する"
date = 2026-03-24
weight = 2
+++

# ドメインとアクションの設定

**ドメイン**とは機能的な範囲（アプリケーション、サービス、操作カテゴリ）のことです。各ドメインには**アクション**（許可されたコマンド）が含まれます。

## デプロイメントドメインを追加する

```toml
[domains.monapp]
description = "Application web principale"

[domains.monapp.actions.deploy]
description = "Déployer une version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy-monapp.sh {tag}"

[domains.monapp.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.monapp.actions.status]
description = "Vérifier l'état du service"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-monapp.sh"

[domains.monapp.actions.restart]
description = "Redémarrer le service"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-monapp.sh"
```

使用方法：

```bash
# stableバージョンをデプロイ
{ echo "monapp deploy version=stable"; echo "."; } | ssh ops@serveur

# 状態を確認
{ echo "monapp status"; echo "."; } | ssh monitoring@serveur

# 再起動
{ echo "monapp restart"; echo "."; } | ssh ops@serveur
```

## バックアップドメインを追加する

```toml
[domains.backup]
description = "Sauvegardes automatisées"

[domains.backup.actions.full]
description = "Sauvegarde complète"
level = "ops"
timeout = 1800
execute = "sudo /usr/local/bin/backup-full.sh {domain}"

[domains.backup.actions.config-only]
description = "Sauvegarde de la configuration"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-config.sh {domain}"
```

## 通知ドメインを追加する

```toml
[domains.notify]
description = "Notifications"

[domains.notify.actions.slack]
description = "Envoyer une notification Slack"
level = "ops"
timeout = 30
execute = "/usr/local/bin/notify-slack.sh {channel} {message}"

[domains.notify.actions.slack.args]
channel = { type = "enum", values = ["general", "ops", "alerts"], default = "ops" }
message = { free = true }
```

引数`message`は`free = true`で宣言されており、任意のテキスト値を受け付けます。

```bash
{ echo 'notify slack channel=ops message="Déploiement terminé"'; echo "."; } | ssh ops@serveur
```

## メンテナンスドメインを追加する

```toml
[domains.infra]
description = "Infrastructure serveur"

[domains.infra.actions.healthcheck]
description = "Vérification de santé des services"
level = "read"
timeout = 30
execute = "/usr/local/bin/healthcheck.sh"

[domains.infra.actions.disk-usage]
description = "Espace disque"
level = "read"
timeout = 10
execute = "/usr/bin/df -h"

[domains.infra.actions.logs]
description = "Derniers logs système"
level = "ops"
timeout = 30
execute = "sudo /usr/bin/journalctl -n 100 --no-pager"
```

## アクション追加後のチェックリスト

1. TOMLの構文を確認（エラーがあるとfail-fast、コード129）
2. 必要に応じて実行スクリプトを作成
3. コマンドが`sudo`を使用する場合はsudoersに追加
4. 別のターミナルから`ssh user@serveur`でテスト
5. `/var/log/ssh-frontiere/commands.json`のログを確認

## ディスカバリ

`help`と`list`コマンドで利用可能なアクションを確認できます：

```bash
# 説明付き完全リスト（#>経由の人間が読めるテキスト）
{ echo "help"; echo "."; } | ssh user@serveur

# ドメインの詳細（#>経由の人間が読めるテキスト）
{ echo "help monapp"; echo "."; } | ssh user@serveur

# JSON形式の短いリスト（ドメイン+アクション）
{ echo "list"; echo "."; } | ssh user@serveur
```

`help`は人間が読めるテキスト（プレフィックス`#>`）を返します。`list`は構造化JSONを返します — 自動解析に適しています。どちらもクライアントの実効レベルでアクセス可能なアクションのみを表示します。

---

**次へ**：[トークンとセキュリティレベル](@/guides/tokens.md) — 誰が何をできるかを制御する。
