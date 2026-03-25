+++
title = "貢献"
description = "SSH-Frontièreへの貢献方法：プロセス、要件、規約"
date = 2026-03-24
weight = 6
+++

# SSH-Frontièreへの貢献

人工知能による支援や生成を含むあらゆる貢献を歓迎します。SSH-Frontière自体がClaude Codeエージェントによって開発されています。

## 始める前に

提案する変更について**イシュー**を作成してください。不要な作業を避け、アプローチを検証するためです。

- **バグ**：観察された動作と期待される動作、バージョン、OSを記述
- **機能**：ユースケースと計画されたアプローチを記述
- **アーキテクチャ変更**：ADRが必要です（`docs/decisions/`を参照）

## プロセス

```
1. イシュー    → 変更について議論
2. フォーク    → git checkout -b feature/my-contribution
3. TDD        → RED（失敗するテスト）→ GREEN（最小限のコード）→ リファクタリング
4. 検証       → make lint && make test && make audit
5. プルリクエスト → 記述し、イシューを参照し、CIがグリーンであること
```

## 品質要件

SSH-Frontièreはセキュリティコンポーネントです。要件は厳格です：

| ルール | 詳細 |
|--------|------|
| テストカバレッジ | 追加コードに対して最低90% |
| `unwrap()`禁止 | `expect()`と`// INVARIANT:`、または`?` / `map_err()`を使用 |
| `unsafe`禁止 | `#[deny(unsafe_code)]`により禁止 |
| 最大800行 | ソースファイルごと |
| 最大60行 | 関数ごと |
| フォーマット | `cargo fmt`必須 |
| lint | `cargo clippy -- -D warnings`（ペダンティック） |

### 依存関係

**不要な依存関係ゼロ。** 新しい依存関係を提案する前に：

1. Rust標準ライブラリでニーズを満たせないか確認
2. 依存関係マトリックスで評価（最低スコア3.5/5）
3. 評価を`docs/searches/`に文書化

現在許可されている依存関係：`serde`、`serde_json`、`toml`。

## コミット規約

メッセージは**英語**、フォーマットは`type(scope): description`：

- `feat(protocol): add TLS support`
- `fix(dispatch): handle empty arguments`
- `test(integration): add session timeout scenarios`
- `docs(references): update configuration guide`

タイプ：`feat`、`fix`、`refactor`、`test`、`docs`。

## AIによる貢献

AIによって生成された貢献は、人間の貢献と同じ条件で受け入れられます：

- 人間の貢献者がコードの品質に**責任を負い続ける**
- テストとlintの要件は同じ
- AIコードを使用した場合はPRに明記する（透明性）

## セキュリティ

### 脆弱性の報告

**公開イシューで脆弱性を報告しないでください。** 責任ある開示のために、メンテナーに直接連絡してください。

### 強化されたレビュー

以下のファイルに影響するPRは、強化されたセキュリティレビューを受けます：

- `protocol.rs`、`crypto.rs` — 認証
- `dispatch.rs`、`chain_parser.rs`、`chain_exec.rs` — コマンドのパースと実行
- `config.rs` — 設定管理

## 最初の貢献に適したもの

- ドキュメントの改善
- エッジケースのテスト追加
- clippy警告の修正
- エラーメッセージの改善

## ライセンス

SSH-Frontièreは[EUPL-1.2](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)の下で配布されています。プルリクエストを提出することで、あなたの貢献がこのライセンスの条件の下で配布されることに同意したものとみなされます。

詳細については、リポジトリ内の[CONTRIBUTING.md](https://github.com/nothus-forge/ssh-frontiere/blob/main/CONTRIBUTING.md)ファイルを参照してください。
