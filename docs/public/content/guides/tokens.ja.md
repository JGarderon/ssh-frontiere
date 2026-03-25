+++
title = "トークンとセキュリティ"
description = "SSH-FrontièreでRBAC認証をトークンで設定する"
date = 2026-03-24
weight = 3
+++

# トークンとセキュリティ

SSH-Frontièreは2つの補完的なアクセス制御メカニズムを提供します：**ベースレベル**（`authorized_keys`経由）と**トークンによるレベル昇格**（ヘッダプロトコル経由）です。

## authorized_keysによるベースレベル

各SSH鍵には`authorized_keys`で定義された固定の信頼レベルがあります：

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-key
```

このレベルは**最低保証**です：`--level=read`のクライアントは`read`レベルのアクションにのみアクセスできます。

## トークンによるレベル昇格

クライアントはトークンで認証することでベースレベル以上に昇格できます。実効レベルは`max(ベースレベル, トークンレベル)`になります。

### トークンを設定する

```toml
[auth]
challenge_nonce = false    # アンチリプレイモードにはtrueを設定

[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]
```

### シークレットを生成する

```bash
# ランダムなシークレットを生成
head -c 32 /dev/urandom | base64
# 結果: "dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ=" のようなもの

# config.tomlに:
# secret = "b64:dGhpcyBpcyBhIHJhbmRvbSBzZWNyZXQ="
```

### トークンを使用する

認証は設定に応じて2つのモードで動作します：

**シンプルモード**（`challenge_nonce = false`、デフォルト）：

1. クライアントがproofを計算：`SHA-256(secret)`
2. クライアントがヘッダを送信：`+ auth token=runner-ci proof=...`

**ノンスモード**（`challenge_nonce = true`）：

1. サーバーがバナーでノンスを送信：`+> challenge nonce=a1b2c3...`
2. クライアントがproofを計算：`SHA-256(XOR_encrypt(secret || nonce, secret))`
3. クライアントがヘッダを送信：`+ auth token=runner-ci proof=...`

```bash
# 補助バイナリでproofを計算
# シンプルモード（ノンスなし）:
PROOF=$(proof --secret "mon-secret")
# ノンスモード:
PROOF=$(proof --secret "mon-secret" --nonce "a1b2c3...")

# 認証付きで送信
{
  echo "+ auth token=runner-ci proof=$PROOF"
  echo "forgejo deploy version=stable"
  echo "."
} | ssh forge-runner@serveur
```

## 可視性タグ

タグはアクションへのアクセスを水平方向にフィルタリングします。`forgejo`タグを持つトークンは、`ops`レベルであっても`forgejo`タグが付いたアクションのみにアクセスできます。

```toml
# タグ付きトークン
[auth.tokens.runner-ci]
secret = "b64:c2VjcmV0LXJ1bm5lci1jaQ=="
level = "ops"
tags = ["forgejo"]

# タグ付きアクション
[domains.forgejo.actions.deploy]
description = "Déploiement"
level = "ops"
execute = "sudo /usr/local/bin/deploy.sh {domain}"
tags = ["forgejo", "deploy"]
```

アクセスルール：
- **タグなしのアクション**：すべてのユーザーがアクセス可能（レベルが十分な場合）
- **タグ付きのアクション**：アイデンティティと少なくとも1つの共通タグがある場合にアクセス可能
- セッション中、複数のトークンのタグは加算（和集合）される

## アンチリプレイのノンスモード

デフォルト（`challenge_nonce = false`）では、proofは単純な`SHA-256(secret)`であり、ノンスは使用されません。`challenge_nonce = true`を有効にすると、サーバーはバナーでノンスを送信し、proofにこのノンスが組み込まれます。ノンスは認証成功ごとに再生成されるため、傍受されたproofのリプレイが防止されます。

```toml
[auth]
challenge_nonce = true
```

このモードは、SSH外のアクセス（TCP直接接続）やチャネルがエンドツーエンドで暗号化されていない場合に推奨されます。

## 不正利用からの保護

| 保護 | 設定 | デフォルト |
|------|------|-----------|
| N回失敗後のロックアウト | `max_auth_failures` | 3 |
| IPバン | `ban_command` | 無効 |
| セッションタイムアウト | `timeout_session` | 3600秒 |

```toml
[global]
max_auth_failures = 3
ban_command = "/usr/sbin/iptables -A INPUT -s {ip} -j DROP"
```

3回の認証失敗後、接続が切断されます。`ban_command`が設定されている場合、送信元IPがバンされます。

---

**次へ**：[AIエージェントとSSH-Frontièreの使用](@/guides/agents-ia.md) — LLM用の制御されたアクセスを設定する。
