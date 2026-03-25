+++
title = "コンパイル"
description = "ソースからSSH-Frontièreをコンパイルする"
date = 2026-03-24
weight = 2
+++

# ソースからのコンパイル

## リリースコンパイル

```bash
# make経由（推奨）
make release

# またはcargoで直接
cargo build --release --target x86_64-unknown-linux-musl
```

生成されるバイナリの場所：

```
target/x86_64-unknown-linux-musl/release/ssh-frontiere
```

これはシステム依存関係のない約1 MBの**静的バイナリ**です。任意のLinux x86_64サーバーに直接コピーできます。

## 検証

```bash
# バイナリタイプの確認
file target/x86_64-unknown-linux-musl/release/ssh-frontiere
# ELF 64-bit LSB executable, x86-64, statically linked

# サイズの確認
ls -lh target/x86_64-unknown-linux-musl/release/ssh-frontiere
# 約1〜2 MB
```

## デバッグコンパイル

開発用：

```bash
make build
# または
cargo build
```

## テスト

デプロイ前にテストが通ることを確認してください：

```bash
# 単体テストと統合テスト
make test

# lint（フォーマット + clippy）
make lint

# 依存関係の監査
make audit
```

## 補助バイナリ：proof

認証プルーフの計算用の補助バイナリが含まれています：

```bash
cargo build --release --target x86_64-unknown-linux-musl --bin proof
```

このバイナリは、クライアント側でSHA-256の計算を実装せずにチャレンジ・レスポンス認証をテストするのに役立ちます。

---

**次へ**：[設定](@/installation/configuration.md) — `config.toml`ファイルの準備。
