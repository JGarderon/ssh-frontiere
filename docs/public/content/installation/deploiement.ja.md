+++
title = "デプロイ"
description = "サーバーにSSH-Frontièreを本番デプロイする"
date = 2026-03-24
weight = 4
+++

# デプロイ

SSH-Frontièreのデプロイは4つのステップで行います：バイナリのインストール、SSH鍵の設定、ログインシェルの変更、sudoersでのセキュリティ設定。

## 1. バイナリのインストール

```bash
# バイナリをサーバーにコピー
scp target/x86_64-unknown-linux-musl/release/ssh-frontiere root@server:/usr/local/bin/

# サーバー上で
chmod 755 /usr/local/bin/ssh-frontiere
```

## 2. 設定のインストール

```bash
# ディレクトリの作成
mkdir -p /etc/ssh-frontiere

# 設定のコピー
cp config.toml /etc/ssh-frontiere/config.toml

# パーミッションの保護（サービスアカウントが設定を読めるようにする）
chown root:forge-runner /etc/ssh-frontiere/config.toml
chmod 640 /etc/ssh-frontiere/config.toml

# ログディレクトリの作成
mkdir -p /var/log/ssh-frontiere
chown forge-runner:forge-runner /var/log/ssh-frontiere
chmod 755 /var/log/ssh-frontiere
```

## 3. サービスアカウントの作成

```bash
# ssh-frontièreをログインシェルとしてユーザーを作成
useradd -m -s /usr/local/bin/ssh-frontiere forge-runner
```

または、アカウントが既に存在する場合：

```bash
# ログインシェルの変更
chsh -s /usr/local/bin/ssh-frontiere forge-runner
```

**注意**：別のセッションからSSH接続が動作することを確認するまで、現在のセッションを閉じないでください。

## 4. SSH鍵の設定（第1層）

`~forge-runner/.ssh/authorized_keys`を編集：

```
# CIランナー鍵（opsレベル）
command="/usr/local/bin/ssh-frontiere --level=ops",restrict ssh-ed25519 AAAA... runner-ci

# モニタリング鍵（読み取り専用レベル）
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... monitoring

# 管理者鍵（adminレベル）
command="/usr/local/bin/ssh-frontiere --level=admin",restrict ssh-ed25519 AAAA... admin-deploy
```

`command=`オプションは、クライアントが送信したコマンドに関係なく、選択した`--level`で`ssh-frontiere`の実行を強制します。`restrict`オプションはポートフォワーディング、エージェントフォワーディング、PTY、X11を無効にします。

```bash
# パーミッションの保護
chmod 700 ~forge-runner/.ssh
chmod 600 ~forge-runner/.ssh/authorized_keys
chown -R forge-runner:forge-runner ~forge-runner/.ssh
```

## 5. sudoersの設定（第3層）

`/etc/sudoers.d/ssh-frontiere`を作成：

```
# SSH-Frontière：サービスアカウントの許可コマンド
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/backup-config.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/deploy.sh *
forge-runner ALL=(root) NOPASSWD: /usr/local/bin/healthcheck.sh
```

ワイルドカード`*`は引数を受け取るスクリプト（例：`backup-config.sh forgejo`）に必要です。引数のないスクリプト（`healthcheck.sh`など）には不要です。

構文の検証：

```bash
visudo -c -f /etc/sudoers.d/ssh-frontiere
```

## 6. 確認

```bash
# 別のターミナルからテスト（現在のセッションを閉じないでください）

# 利用可能なコマンドが表示されることを確認
{ echo "help"; echo "."; } | ssh forge-runner@server

# コマンドのテスト
{ echo "infra healthcheck"; echo "."; } | ssh forge-runner@server
```

## 多層防御

3つの層が互いに補完します：

| 層 | メカニズム | 防御内容 |
|----|-----------|----------|
| 1 | `authorized_keys`の`command=` + `restrict` | レベルを強制、フォワーディング/PTYをブロック |
| 2 | SSH-Frontière（ログインシェル） | TOMLホワイトリストに対して検証 |
| 3 | sudoersの`sudo` | システムコマンドを制限 |

攻撃者がSSH鍵を侵害しても、ホワイトリストで許可されたコマンドしか実行できません。第2層を迂回しても、権限はsudoersで制限されます。

## ロールバック

何か問題がある場合は、通常のシェルに戻します：

```bash
# コンソール（IPMI/KVM）または別の管理者アカウント経由
chsh -s /bin/bash forge-runner
```

**ヒント**：ログインシェルを変更する前に`/etc/passwd`をバックアップしてください。

```bash
cp /etc/passwd /etc/passwd.bak.$(date +%Y%m%d)
```

---

**次へ**：[はじめての使い方](@/guides/premier-usage.md) — SSH-Frontière経由での最初のSSHコマンド。
