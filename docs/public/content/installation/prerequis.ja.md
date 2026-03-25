+++
title = "前提条件"
description = "SSH-Frontièreのインストールに必要なもの"
date = 2026-03-24
weight = 1
+++

# 前提条件

## 対象サーバー

| 要素 | 詳細 |
|------|------|
| システム | Linux x86_64 |
| SSHアクセス | 動作する`sshd` |
| サービスアカウント | 専用ユーザー（例：`forge-runner`） |
| 救済用管理者アカウント | `/bin/bash`を持つアカウント（変更しない） |
| コンソールアクセス | IPMI、KVM、またはクラウドコンソール — SSHロックアウト時のため |

**重要**：動作するコンソールアクセスと通常のシェルを持つ管理者アカウントを常に維持してください。SSH-Frontièreのログインシェルが誤設定された場合、サービスアカウントへのSSHアクセスを失う可能性があります。

## ビルドマシン

SSH-Frontièreをソースからコンパイルするには：

| 要素 | 詳細 |
|------|------|
| Rust | バージョン1.70以上 |
| muslターゲット | `x86_64-unknown-linux-musl`（静的バイナリ用） |
| `make` | オプション、Makefileのショートカット用 |

### muslターゲットのインストール

```bash
rustup target add x86_64-unknown-linux-musl
```

## 代替手段：コンパイル済みバイナリ

コンパイルしたくない場合は、[プロジェクトのリリースページ](https://github.com/nothus-forge/ssh-frontiere/releases)から静的バイナリをダウンロードできます。バイナリにはシステム依存関係がありません。

---

**次へ**：[ソースからのコンパイル](@/installation/compilation.md)
