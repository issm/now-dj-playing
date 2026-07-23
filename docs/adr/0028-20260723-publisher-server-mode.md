# ADR-0028: Publisher の ndp-server 対応（設定ファイル導入・Web モード追加）

## ステータス

Accepted

## コンテキスト

Phase 1 の publisher CLI (`ndp-publish`) はローカルファイルシステムへの書き出しのみに対応しており、すべてのパラメータを CLI 引数で渡す設計だった。Phase 2 でインターネット上の ndp-server に HTTP で楽曲情報を送信する機能が必要になり、以下の課題が生じた:

- 毎回変わらないパラメータ（`dj_name`, 出力先ディレクトリ, サーバ URL 等）を都度指定するのは煩雑
- one-shot CLI であるため、セッショントークンをプロセス間で共有する仕組みが必要
- 既存の local モード動作を壊さずに web モードを追加したい

## 決定

### 1. 設定ファイルの導入

JSONC 形式の設定ファイルを導入し、安定的なパラメータを集約する。

#### 設定ファイルの構造

```jsonc
{
  // DJ 名 (テキスト) またはロゴ画像パス (png/jpg/jpeg)
  "dj_name": "DJ サンプル",
  // local モード向け
  "local": {
    "dj_id": "dj-000",
    // 出力先ベースディレクトリ（~ はホームディレクトリに展開される）
    "publish_base_dir": "~/tmp/ndp"
  },
  // web モード向け
  "web": {
    "endpoint_url": "http://localhost:8080/api"
  }
}
```

#### ルックアップ順

設定ファイルは以下の順に探索し、最初に見つかったものを使用する:

1. CLI 引数 `-c, --config-file` で指定されたパス
2. 環境変数 `NDP_PUBLISH_CONFIG` で指定されたパス
3. カレントディレクトリの `ndp-publish.config.json`
4. `$HOME/.config/ndp/publish.config.json`

いずれも見つからない場合は空の設定で続行する（`--out` 等の CLI 引数のみで動作可能）。

`-c` 未指定でルックアップにより設定ファイルが見つかった場合は、読み込んだファイルのパスを stderr に表示する。

#### パス指定

`-c` および環境変数には相対パスを指定できる。内部的に `canonicalize` で絶対パスに解決して保持するため、トークンファイルの配置先や設定内の相対パス解決が正しく機能する。

#### パス解決（設定ファイル内の値）

- `~` はホームディレクトリに展開する（`shellexpand::tilde`）
- 展開後が相対パスの場合、設定ファイルの親ディレクトリを基準に解決する（viewer の ADR-0021 と同方針）

### 2. モード切り替え

`--mode local|web` オプションでモードを指定する。デフォルトは `local`（既存互換）。

| モード | 動作 |
|---|---|
| `local` | 従来通りファイルシステムに書き出す |
| `web` | ndp-server に HTTP POST で送信する |

両モードの同時指定は将来的な拡張候補だが、初期実装ではいずれか一方のみとする。

### 3. CLI オプションの整理

#### 新規追加

| オプション | 必須 | 説明 |
|---|---|---|
| `-c, --config-file` | - | 設定ファイルパス |
| `-m, --mode` | - | `local` (default) / `web` |
| `--code` | web 初回のみ | セッション参加用 6 桁コード |

#### 既存オプションの扱い

| オプション | 変更 |
|---|---|
| `--file` | 変更なし（必須） |
| `--out` | 設定ファイルの `local.publish_base_dir` をデフォルト値として使用。CLI 指定時はオーバーライド |
| `--id` | 設定ファイルの `local.dj_id` をデフォルト値として使用。CLI 指定時はオーバーライド |
| `--dj-name` | 設定ファイルの `dj_name` をデフォルト値として使用。CLI 指定時はオーバーライド |

CLI 引数は設定ファイルの値よりも優先される（override 方式）。

### 4. Web モードの動作フロー

```
1. 設定ファイルを読み込む
2. セッショントークンの確認:
   a. トークンファイルが存在する → 再利用
   b. 存在しない or --code が指定された → POST /api/sessions/join で新規参加
3. 楽曲ファイルからタグを抽出
4. アートワークを Base64 Data URI にエンコード
5. POST /api/sessions/{session_id}/publish で送信
```

### 5. トークン管理

#### 保存先

設定ファイルと同じディレクトリに `.ndp-session.json` を自動生成する。

例: 設定ファイルが `~/apps/ndp/ndp-publish.config.json` の場合 → `~/apps/ndp/.ndp-session.json`

#### トークンファイルの構造

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "code": "037482",
  "publisher_id": "pub_001",
  "token": "pt_xxxxxxxxxxxx"
}
```

#### 再 join の条件

- トークンファイルが存在しない
- `--code` が CLI で指定され、かつ保存済みの code と異なる

トークンファイルが存在し `--code` 未指定の場合は保存済みトークンを再利用する。

### 6. アートワークの扱い

- Base64 Data URI (`data:image/jpeg;base64,...`) としてリクエストボディに含める
- ADR-0025 の publish API 仕様に準拠
- サイズ制限は設けない（将来必要になれば別 issue で対応）

### 7. エラーハンドリング

- リトライは行わない。失敗時は stderr にメッセージを出力して exit 1
- publisher は one-shot CLI であり、次の曲変更時に再実行されるため自然に復帰する
- ネットワークエラー、認証エラー (401/403)、サーバエラー (5xx) はすべて即座にエラー終了

## 理由

- **方針 A (既存 CLI にフラグ追加) を採用**: publisher は「1 曲ごとに 1 回実行する one-shot CLI」という性質を持つ。別バイナリ化 (B) やブリッジプロセス (C) はプロセス管理の複雑さが増すだけで、この性質と合わない
- **設定ファイル導入**: `dj_name`, `publish_base_dir`, `endpoint_url` 等は曲ごとに変わらない。毎回 CLI 引数で渡すのは DJ ソフトのフック設定が煩雑になる
- **トークンのファイル永続化**: one-shot CLI はプロセス間でメモリを共有できないため、ファイルが唯一の共有手段
- **リトライなし**: DJ ソフトのフックをブロックしないことを優先。次の曲変更で自然に復帰する設計

## 影響

- `apps/publisher/Cargo.toml` に HTTP クライアント (`reqwest`)、JSONC パーサー (`serde_json_lenient`)、`shellexpand`、`base64` の依存追加
- 設定ファイルなしでも `--out` と `--file` を指定すれば従来通り動作する（後方互換）
- viewer 側の設定ファイル (`ndp.config.json`) とは別ファイル（publisher 専用）
