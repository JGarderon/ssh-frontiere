+++
title = "CI/CD統合"
description = "Forgejo ActionsまたはGitHub ActionsからSSH-Frontière経由でデプロイする"
date = 2026-03-24
weight = 5
+++

# CI/CD統合

SSH-FrontièreはCI/CDパイプラインと自然に統合できます。ランナーがSSH経由でコマンドを送信し、SSH-Frontièreが検証して実行します。

## Forgejo Actions

### 前提条件

1. ランナー用の専用SSH鍵（`authorized_keys`で`--level=ops`を設定）
2. Forgejoリポジトリにシークレットとして秘密鍵を保存（`SSH_PRIVATE_KEY`）
3. サーバーアドレスをシークレットとして保存（`DEPLOY_HOST`）

### デプロイメントワークフロー

```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configurer la clé SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Déployer
        run: |
          {
            echo "forgejo deploy version=latest"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}

      - name: Vérifier
        run: |
          RESULT=$({
            echo "infra healthcheck"
            echo "."
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
```

## GitHub Actions

### 同等のワークフロー

```yaml
name: Deploy via SSH-Frontière
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Configurer la clé SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.SSH_PRIVATE_KEY }}" > ~/.ssh/deploy-key
          chmod 600 ~/.ssh/deploy-key
          ssh-keyscan -H ${{ secrets.DEPLOY_HOST }} >> ~/.ssh/known_hosts

      - name: Déployer via SSH-Frontière
        run: |
          {
            echo "monapp deploy version=stable"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }}

      - name: Vérifier le déploiement
        run: |
          RESULT=$({
            echo "monapp status"
            echo "."
          } | ssh -i ~/.ssh/deploy-key deploy@${{ secrets.DEPLOY_HOST }})
          echo "$RESULT"
          # 最終JSONレスポンス（>>>プレフィックス）を解析
          STATUS=$(echo "$RESULT" | grep "^>>> " | sed 's/^>>> //' | python3 -c "import sys,json; print(json.load(sys.stdin)['status_code'])")
          if [ "$STATUS" != "0" ]; then
            echo "Déploiement échoué (code $STATUS)"
            exit 1
          fi
```

## CI/CD用サーバー設定

### 典型的なアクション

```toml
[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.deploy]
description = "Déployer une version"
level = "ops"
timeout = 300
execute = "sudo /usr/local/bin/deploy.sh {domain} {tag}"

[domains.forgejo.actions.deploy.args]
tag = { type = "enum", values = ["latest", "stable", "canary"], default = "latest" }

[domains.forgejo.actions.rollback]
description = "Revenir à la version précédente"
level = "ops"
timeout = 120
execute = "sudo /usr/local/bin/rollback.sh {domain}"

[domains.forgejo.actions.backup-pre-deploy]
description = "Sauvegarde avant déploiement"
level = "ops"
timeout = 600
execute = "sudo /usr/local/bin/backup-pre-deploy.sh {domain}"
```

### ランナーのSSH鍵

```
# deployアカウントのauthorized_keys
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... forgejo-runner
```

### Sudoers

```
deploy ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/rollback.sh *
deploy ALL=(root) NOPASSWD: /usr/local/bin/backup-pre-deploy.sh *
```

ワイルドカード`*`は、SSH-Frontièreが解決済みの引数をスクリプトに渡すため必要です（例：`deploy.sh forgejo latest`）。

## 複数ステップのパイプライン

完全なデプロイメント（バックアップ、デプロイ、検証、通知）の場合：

```yaml
      - name: Pipeline complet (backup, deploy, verify)
        run: |
          {
            echo "+ session keepalive"
            echo "forgejo backup-pre-deploy"
            echo "."
            echo "forgejo deploy version=stable"
            echo "."
            echo "infra healthcheck"
            echo "."
            echo "."   # 空のブロック = セッション終了
          } | ssh -i ~/.ssh/deploy-key forge-runner@${{ secrets.DEPLOY_HOST }}
```

各コマンドの後に`.`（ブロック終了）が続きます。コマンドなしの`.`はセッション終了を示します。セッションモードにより、コマンドごとにSSH接続を開く必要がなくなります。

## ベストプラクティス

1. **パイプラインごとに専用鍵**：ランナー/ワークフローごとに1つのSSH鍵、必要最小限のレベル
2. **シークレット管理**：秘密鍵はコードに保存しない — CIのシークレット機能を使用
3. **デプロイ前バックアップ**：デプロイ前に必ずバックアップを取得
4. **デプロイ後検証**：デプロイ後にヘルスチェックを実行
5. **ロールバック**：迅速に元に戻すためのロールバックアクションを用意
6. **ログ**：SSH-FrontièreのJSONログで各デプロイメントを追跡可能

---

**参照**：[FAQ](@/faq.md) | [代替手段](@/alternatives.md) | [コントリビュート](@/contribuer.md)
