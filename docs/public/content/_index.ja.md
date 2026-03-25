+++
title = "SSH-Frontière"
description = "Rust製の制限付きSSHログインシェル — 受信接続の宣言的制御"
sort_by = "weight"

[extra]
framed = true
+++

# SSH-Frontière

**Rust製の制限付きSSHログインシェル** — すべての受信SSH接続に対する安全な単一のエントリポイント。

SSH-Frontièreは、Unixアカウントのデフォルトシェル（`/bin/bash`）を、宣言的なTOML設定ファイルに基づいて**すべてのコマンドを検証**してから実行するプログラムに置き換えます。

[![GitHub](https://img.shields.io/badge/GitHub-Open--source_repository-181717?logo=github&logoColor=white&style=for-the-badge)](https://github.com/JGarderon/ssh-frontiere)

---

## なぜSSH-Frontièreなのか？

**デフォルトで安全** — 明示的に許可されていないコマンドは実行されません。デフォルト拒否、シェルなし、インジェクション不可能。

**デプロイが簡単** — 約1 MBの静的バイナリ、TOMLファイル1つ、`/etc/passwd`に1行追加するだけ。デーモンもサービス管理も不要。

**柔軟性** — 3段階のアクセスレベル（read、ops、admin）、可視性タグ、構造化ヘッダプロトコル。AIエージェント、CI/CDランナー、メンテナンススクリプトに対応。

**監査可能** — 実行または拒否されたすべてのコマンドが構造化JSONでログに記録されます。399のcargoテスト＋72のE2E SSHシナリオ。

---

## ユースケース

- **CI/CDランナー**（Forgejo Actions、GitHub Actions）：SSH経由でのデプロイ、バックアップ、ヘルスチェック
- **AIエージェント**（Claude Codeなど）：信頼レベルに基づくサーバーリソースへの制御されたアクセス
- **自動メンテナンス**：バックアップ、監視、通知スクリプト

---

## 概要

| | |
|---|---|
| **言語** | Rust（静的muslバイナリ、約1 MB） |
| **ライセンス** | [EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12) — 欧州連合パブリックライセンス |
| **テスト** | 399 cargo + 72 E2E SSH + 9 fuzzハーネス |
| **依存関係** | 直接クレート3つ（`serde`、`serde_json`、`toml`） |
| **設定** | 宣言的TOML |
| **プロトコル** | stdin/stdout上のテキストヘッダ、JSONレスポンス |

---

## はじめに

- [SSH-Frontièreを知る](@/presentation.md) — 何であり、何をし、なぜ存在するのか
- [インストール](@/installation/_index.md) — コンパイル、設定、デプロイ
- [ガイド](@/guides/_index.md) — ステップバイステップのチュートリアル
- [セキュリティ](@/securite.md) — セキュリティモデルと保証
- [アーキテクチャ](@/architecture.md) — 技術設計
- [代替手段](@/alternatives.md) — 他のソリューションとの比較
- [FAQ](@/faq.md) — よくある質問
- [貢献](@/contribuer.md) — プロジェクトに参加する
