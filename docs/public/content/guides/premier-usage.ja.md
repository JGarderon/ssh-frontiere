+++
title = "はじめての使い方"
description = "SSH-Frontièreのインストール、最初のドメイン設定、テスト"
date = 2026-03-24
weight = 1
+++

# はじめての使い方

このガイドでは、インストールからSSH-Frontière経由での最初のSSHコマンド実行までを説明します。

## 1. 最小限の設定を準備する

最小限の`config.toml`ファイルを作成します：

```toml
[global]
log_file = "/var/log/ssh-frontiere/commands.json"
default_timeout = 60

[domains.test]
description = "Domaine de test"

[domains.test.actions.hello]
description = "Commande de test qui affiche un message"
level = "read"
timeout = 10
execute = "/usr/bin/echo hello from ssh-frontiere"
```

この設定では、`read`レベルでアクセス可能な`hello`アクションを持つ`test`ドメインを1つ定義しています。

## 2. インストールと設定

まず`ssh-frontiere`バイナリを用意する必要があります。[コンパイルガイド](@/installation/compilation.md)を参照するか、[リリースページ](https://github.com/nothus-forge/ssh-frontiere/releases)からプリコンパイル済みバイナリをダウンロードしてください。

```bash
# バイナリをコピー
sudo cp ssh-frontiere /usr/local/bin/
sudo chmod 755 /usr/local/bin/ssh-frontiere

# 設定をインストール
sudo mkdir -p /etc/ssh-frontiere
sudo cp config.toml /etc/ssh-frontiere/config.toml
sudo chmod 640 /etc/ssh-frontiere/config.toml

# ログディレクトリを作成
sudo mkdir -p /var/log/ssh-frontiere

# サービスアカウントを作成
sudo useradd -m -s /usr/local/bin/ssh-frontiere test-user

# ログへの書き込み権限を付与
sudo chown test-user:test-user /var/log/ssh-frontiere
```

## 3. SSH鍵を設定する

クライアントマシンで：

```bash
# 鍵を生成
ssh-keygen -t ed25519 -C "test-key" -f ~/.ssh/test-frontiere
```

サーバーで、公開鍵を`~test-user/.ssh/authorized_keys`に追加します：

```
command="/usr/local/bin/ssh-frontiere --level=read",restrict ssh-ed25519 AAAA... test-key
```

```bash
# パーミッションを確保
sudo chmod 700 ~test-user/.ssh
sudo chmod 600 ~test-user/.ssh/authorized_keys
sudo chown -R test-user:test-user ~test-user/.ssh
```

## 4. 最初の呼び出し

```bash
# 利用可能なコマンドを確認
{ echo "help"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

期待されるレスポンス（サーバーはまずバナーを送信し、その後レスポンスを返します）：

```
#> ssh-frontiere 0.1.0
+> capabilities session, help, body
#> type "help" for available commands
#> ...
>>> {"command":"help","status_code":0,"status_message":"ok","stdout":null,"stderr":null}
```

`#>`で始まる行は人間が読めるヘルプテキストです。`help`コマンドは`read`レベルでアクセス可能なドメインとアクションの一覧を表示します。

## 5. コマンドを実行する

```bash
{ echo "test hello"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

期待されるレスポンス：

```
>> hello from ssh-frontiere
>>> {"command":"test hello","status_code":0,"status_message":"executed","stdout":null,"stderr":null}
```

プログラムの出力（`hello from ssh-frontiere`）は`>>`でストリーミング送信され、最終的なJSONレスポンスは`>>>`で送信されます。JSONの`stdout`と`stderr`フィールドは出力がストリーミングで送信されたため`null`です。

## 6. フローを理解する

以下が実行された内容です：

1. SSHクライアントが`test-frontiere`鍵で接続
2. `sshd`が鍵を認証し`authorized_keys`を読み取り
3. `command=`オプションが`ssh-frontiere --level=read`の実行を強制
4. SSH-Frontièreがバナー（`#>`、`+>`）を表示し、ヘッダを待機
5. クライアントがコマンド`test hello`（プレーンテキスト、プレフィックスなし）を送信し、`.`（ブロック終了）を送信
6. SSH-Frontièreが検証：ドメイン`test`、アクション`hello`、レベル`read` <= 必要な`read`
7. SSH-Frontièreが`/usr/bin/echo hello from ssh-frontiere`を実行
8. 出力がストリーミング送信（`>>`）され、最終JSONレスポンス（`>>>`）が送信される

## 7. 拒否をテストする

存在しないコマンドを試してみましょう：

```bash
{ echo "test inexistant"; echo "."; } | ssh -i ~/.ssh/test-frontiere test-user@serveur
```

レスポンス：

```
>>> {"command":"test inexistant","status_code":128,"status_message":"rejected: unknown action 'inexistant' in domain 'test'","stdout":null,"stderr":null}
```

コマンドが実行されなかったため、`stdout`と`stderr`は`null`です。

## 次のステップ

SSH-Frontièreが動作するようになりました。次は[独自のドメインとアクションを設定](@/guides/domaines.md)しましょう。
