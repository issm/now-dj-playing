# ADR-0031: ndp-server のアクセスログを JSON 形式で出力する

## ステータス

Accepted

## コンテキスト

ndp-server は `tracing_subscriber::fmt()` のデフォルト（テキスト形式）でログを出力していた。テキスト形式は人間が目視する分には読みやすいが、以下の問題がある:

- ログ集約基盤（CloudWatch Logs, Loki 等）でのパースにカスタムパーサが必要
- フィールド抽出（ステータスコード、レイテンシ等）が正規表現に依存し脆い
- 構造化クエリ（JSON path ベースのフィルタ）が使えない

### 検討した選択肢

| 選択肢 | メリット | デメリット |
|---|---|---|
| **tracing-subscriber の json feature (採用)** | 依存追加のみ。既存の `TraceLayer` がそのまま使える | 開発時の可読性がやや低下する |
| カスタム Layer で独自フォーマット | 出力内容を完全制御可能 | 実装コストが高い。メンテナンス負荷 |
| テキスト形式のまま維持 | 変更不要 | 上記の問題が残る |

## 決定

`tracing-subscriber` に `json` feature を追加し、`tracing_subscriber::fmt().json()` で JSON 形式のログを出力する。

### 変更内容

- `apps/server/Cargo.toml`: `tracing-subscriber` の features に `"json"` を追加
- `apps/server/src/main.rs`: `.json()` メソッドチェーンを追加

### 出力例

```json
{"timestamp":"2026-07-25T12:00:00.123456Z","level":"INFO","fields":{"message":"finished processing request"},"target":"tower_http::trace::on_response","span":{"method":"GET","uri":"/health"},"spans":[...]}
```

## 理由

- `tracing-subscriber` の標準機能であり、追加の依存クレート（`tracing-serde` は内部依存として自動解決）以外のコストがない
- `tower_http::trace::TraceLayer` が発行する span がそのまま JSON フィールドとして出力されるため、カスタムミドルウェアが不要
- 開発時の可読性は `RUST_LOG` でフィルタレベルを調整するか、`jq` で整形すれば対処可能

## 影響

- 全てのログ出力（アクセスログ含むアプリケーションログ）が JSON 形式になる
- 開発時のターミナル出力が JSON になるため、`| jq` でパイプする運用が推奨される
- 将来的に CloudWatch Logs Insights 等で JSON フィールドによる直接クエリが可能になる
