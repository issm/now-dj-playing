# ADR-0024: 設定ファイルパーサーを serde_json_lenient に移行

## ステータス

Accepted

## コンテキスト

ADR-0016 で設定ファイル形式として JSONC（JSON with Comments）を採用し、`//` および `/* */` コメントと trailing comma を許容すると決定した。パーサーとして `serde_jsonc` クレートを選定したが、実際には `serde_jsonc` v1.0.108 は trailing comma をサポートしておらず、パースエラー（`ErrorCode::TrailingComma`）を返す実装であった。

設定ファイルにコメント行をコメントアウトで切り替える運用（例: `"path"` の候補を `//` で無効化）をすると、末尾の有効行の後にカンマが残り、trailing comma エラーが発生するケースがあった（#34）。

## 決定

`serde_jsonc` を `serde_json_lenient`（Google 製、`serde_json` フォーク）に置き換える。

### 選定理由

- `serde_json` からのフォークであり、API が完全互換（`from_str` 等の関数シグネチャが同一）
- コメント（`//`, `/* */`）と trailing comma の両方をサポート
- 各拡張機能がスイッチ可能（デフォルトで全て有効）
- Google がメンテナンスしており、週 14 万超のダウンロード実績がある
- `\v` や `\xDD` エスケープもサポート（将来の柔軟性）

### 不採用とした候補

| クレート | 不採用理由 |
|---|---|
| `serde_jsonc` | trailing comma 非対応（本 ADR の発端） |
| `json5` | JSON5 仕様全体を受け入れることになり、設定の緩さが過剰 |
| `serde_jsonc2` | メンテナンス状況が不明、ダウンロード数が少ない |

### 変更箇所

- `apps/viewer/src-tauri/Cargo.toml`: `serde_jsonc = "1"` → `serde_json_lenient = "0.2"`
- `apps/viewer/src-tauri/src/config.rs`: `serde_jsonc::from_str` → `serde_json_lenient::from_str`

## 影響

- ADR-0016 で記載した「trailing comma を許容する」仕様が実際に機能するようになる
- 設定ファイルのパーサー名称が変わるため、今後の依存更新時は `serde_json_lenient` を対象とする
- `serde_json`（通常の JSON 出力用）は引き続き別途依存として残る
