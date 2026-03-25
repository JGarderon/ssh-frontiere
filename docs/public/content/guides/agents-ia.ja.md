+++
title = "AIエージェント"
description = "SSH-FrontièreをAIエージェント（Claude Codeなど）と使用する"
date = 2026-03-24
weight = 4
+++

# AIエージェントとSSH-Frontièreの使用

SSH-Frontièreは当初からAIエージェント（LLM）との互換性を考慮して設計されています。構造化プロトコル、自動ディスカバリ、JSONレスポンスにより、サーバー上でアクションを実行する必要があるエージェントにとって理想的なエントリポイントとなっています。

## なぜAIエージェントにSSH-Frontièreなのか

AIエージェント（Claude Code、Cursor、GPTなど）はSSH経由でサーバー上のコマンドを実行できます。問題は、制御がなければエージェントが何でも実行できてしまうことです。

SSH-Frontièreはこの問題を解決します：

- **アクションを制限**：エージェントは設定されたコマンドのみ実行可能
- **アクセスレベル**：`read`レベルのエージェントは参照のみ、変更は不可
- **ディスカバリ**：エージェントは`help`を使って利用可能なアクションを確認可能
- **構造化JSON**：レスポンスはエージェントが直接解析可能

## AIエージェント用の設定

### 1. 専用SSH鍵

エージェント用のSSH鍵を生成します：

```bash
ssh-keygen -t ed25519 -C "agent-claude" -f ~/.ssh/agent-claude
```

### 2. 制限された信頼レベル

`authorized_keys`で最小限のレベルを設定します：

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... agent-claude
```

`read`から始めて、必要に応じてトークンで昇格させます。

### 3. 専用ドメイン

エージェント用の特定のアクションを設定します：

```toml
[domains.agent]
description = "Actions pour agents IA"

[domains.agent.actions.status]
description = "État des services"
level = "read"
timeout = 30
execute = "/usr/local/bin/status-all.sh"

[domains.agent.actions.logs]
description = "Derniers logs applicatifs"
level = "read"
timeout = 30
execute = "/usr/local/bin/recent-logs.sh {service}"

[domains.agent.actions.logs.args]
service = { type = "enum", values = ["web", "api", "worker", "database"] }

[domains.agent.actions.restart]
description = "Redémarrer un service"
level = "ops"
timeout = 60
execute = "sudo /usr/local/bin/restart-service.sh {service}"
tags = ["agent-ops"]

[domains.agent.actions.restart.args]
service = { type = "enum", values = ["web", "api", "worker"] }
```

### 4. 昇格用トークン（オプション）

エージェントが`ops`アクションにアクセスする必要がある場合：

```toml
[auth.tokens.agent-claude]
secret = "b64:c2VjcmV0LWFnZW50LWNsYXVkZQ=="
level = "ops"
tags = ["agent-ops"]
```

## Claude Code（AutoClaude）での使用例

AutoClaudeコンテナ内のClaude Codeエージェントは、SSH-Frontièreを使用してホストサーバー上でアクションを実行できます：

```bash
# エージェントが利用可能なコマンドを発見（list経由のJSON）
{ echo "list"; echo "."; } | ssh -i /keys/agent-claude agent@serveur

# エージェントがサービスの状態を確認
{ echo "agent status"; echo "."; } | ssh -i /keys/agent-claude agent@serveur

# エージェントがサービスのログを読み取り
{ echo "agent logs service=api"; echo "."; } | ssh -i /keys/agent-claude agent@serveur
```

出力はストリーミング送信（`>>`）され、その後最終JSONレスポンス（`>>>`）が続きます：

```
>> web: running
>> api: running
>> worker: stopped
>>> {"command":"agent status","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

エージェントは`>>`行（ストリーミングされた標準出力）を解析し、`worker`が停止していることを検出して、それに応じてアクションを決定できます。`>>>`レスポンスはリターンコードを確認します。

## セッションモード

コマンドごとにSSH接続を開くのを避けるため、エージェントはセッションモードを使用できます：

```bash
{
  echo "+ auth token=agent-claude proof=..."
  echo "+ session keepalive"
  echo "agent status"
  echo "."
  echo "agent logs service=worker"
  echo "."
  echo "."   # 空のブロック = セッション終了
} | ssh -i /keys/agent-claude agent@serveur
```

各コマンドの後に`.`（ブロック終了）が続きます。コマンドなしの`.`はセッション終了を示します。セッションモードでは、1つのSSH接続で複数のコマンドを送信でき、グローバルタイムアウト（`timeout_session`）を設定できます。

## ベストプラクティス

1. **最小権限の原則**：`read`から始め、必要な場合のみトークンで昇格
2. **アトミックなアクション**：各アクションは1つのことだけを行う。エージェントがアクションを組み合わせる
3. **明示的な名前**：ドメインとアクション名は`help`で表示される — わかりやすい名前にする
4. **可視性タグ**：専用タグでエージェントのアクションを分離
5. **出力制限**：`max_stdout_chars`を設定してエージェントが大量のデータを受信しないようにする
6. **ログ**：ログを監視して異常な使用を検出する

---

**次へ**：[CI/CD統合](@/guides/ci-cd.md) — SSH-Frontière経由でデプロイメントを自動化する。
