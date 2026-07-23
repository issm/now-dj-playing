# ADR-0028: Publisher の ndp-server 対応（設定ファイル導入・Web モード追加）

## ステータス

Accepted

## コンテキスト

Phase 1 の publisher CLI (`ndp-publish`) はローカルファイルシステムへの書き出しのみに対応しており、すべてのパラメータを CLI 引数で渡す設計だった。Phase 2 でインターネット上の ndp-server に HTTP で楽曲情報を送信する機能が必要になり、以下の課題が生じた:

- 毎回変わらないパラメータ（`dj_name`, 出力先ディレクトリ, サーバ URL 等）を都度指定するのは煩雑
- one-shot CLI であるため、セッション情報をプロセス間で共有する仕組みが必要
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

`-c` および環境変数には相対パスを指定できる。内部的に `canonicalize` で絶対パスに解決して保持するため、セッションファイルの配置先や設定内の相対パス解決が正しく機能する。

#### パス解決（設定ファイル内の値）

- `~` はホームディレクトリに展開する（`shellexpand::tilde`）
- 展開後が相対パスの場合、設定ファイルの親ディレクトリを基準に解決する（viewer の ADR-0021 と同方針）

### 2. モード切り替え

`-W, --web-mode` フラグで web モードを有効化する。未指定時は local モード（既存互換）。

| モード | 動作 |
|---|---|
| local (デフォルト) | 従来通りファイルシステムに書き出す |
| web (`-W`) | ndp-server に HTTP POST で送信する |

### 3. CLI オプションの整理

#### 新規追加

| オプション | 必須 | 説明 |
|---|---|---|
| `-c, --config-file` | - | 設定ファイルパス |
| `-W, --web-mode` | - | web モードで動作する |
| `-C, --code` | web 初回のみ | セッション参加用 6 桁コード |
| `-J, --join-only` | - | join のみ実行して終了する（web モード） |

#### 既存オプションの扱い

| オプション | 変更 |
|---|---|
| `-f, --file` | `-J` 時は不要。それ以外では必須 |
| `-o, --out` | 設定ファイルの `local.publish_base_dir` をデフォルト値として使用。CLI 指定時はオーバーライド |
| `--id` | 設定ファイルの `local.dj_id` をデフォルト値として使用。CLI 指定時はオーバーライド |
| `--dj-name` | 設定ファイルの `dj_name` をデフォルト値として使用。CLI 指定時はオーバーライド |

CLI 引数は設定ファイルの値よりも優先される（override 方式）。

### 4. Web モードの動作フロー

#### 通常の publish (`-W`)

```
1. 設定ファイルを読み込む
2. セッションファイルの確認:
   a. ファイルが存在する → その情報で publish を試行
      - 401 応答 → -C が指定されていれば再 join してセッションファイルを上書き → publish 再試行
      - 401 応答 & -C 未指定 → エラー終了
   b. ファイルが存在しない → -C で join してセッションファイルを作成 → publish
3. 楽曲ファイルからタグを抽出
4. アートワークを Base64 Data URI にエンコード
5. POST /api/sessions/{session_id}/publish で送信
```

#### join のみ (`-W -J`)

```
1. 設定ファイルを読み込む
2. -C で指定されたコードで POST /api/sessions/join
3. セッションファイルを作成して終了
```

### 5. セッション管理

#### セッションファイルの構造

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "publisher_id": "pub_001",
  "token": "pt_xxxxxxxxxxxx"
}
```

#### 配置先ルックアップ

ファイル名: `ndp-publish.session.json`

配置先ディレクトリは以下の順で決定する:

1. 環境変数 `NDP_PUBLISH_SESSION_DIR` で指定されたディレクトリ
2. 設定ファイルと同じディレクトリ

#### 再 join の条件

- セッションファイルが存在しない
- publish が 401 Unauthorized で失敗し、かつ `-C` でコードが指定されている

### 6. アートワークの扱い

- Base64 Data URI (`data:image/jpeg;base64,...`) としてリクエストボディに含める
- ADR-0025 の publish API 仕様に準拠
- サイズ制限は publisher 側では設けない
- ndp-server 側の body size limit (axum デフォルト 2 MiB) を超える場合は 413 エラーとなる（サーバ側で引き上げが必要）

### 7. エラーハンドリング

- publish 時の 401 に限り再 join を試行する（`-C` 指定時のみ）
- それ以外のエラーではリトライを行わない。失敗時は stderr にメッセージを出力して exit 1
- publisher は one-shot CLI であり、次の曲変更時に再実行されるため自然に復帰する
- ネットワークエラー、認証エラー (403)、サーバエラー (5xx) は即座にエラー終了

## 理由

- **方針 A (既存 CLI にフラグ追加) を採用**: publisher は「1 曲ごとに 1 回実行する one-shot CLI」という性質を持つ。別バイナリ化 (B) やブリッジプロセス (C) はプロセス管理の複雑さが増すだけで、この性質と合わない
- **設定ファイル導入**: `dj_name`, `publish_base_dir`, `endpoint_url` 等は曲ごとに変わらない。毎回 CLI 引数で渡すのは DJ ソフトのフック設定が煩雑になる
- **セッション情報のファイル永続化**: one-shot CLI はプロセス間でメモリを共有できないため、ファイルが唯一の共有手段
- **401 時の自動再 join**: セッションがサーバ側で失効した場合（サーバ再起動等）に、手動操作なしで復帰可能にする
- **`-J` (join-only)**: セッション参加とファイル publish を分離することで、DJ ソフトのフックに組み込む前に手動でセッション参加を済ませておくワークフローに対応

## 影響

- `apps/publisher/Cargo.toml` に HTTP クライアント (`reqwest`)、JSONC パーサー (`serde_json_lenient`)、`shellexpand`、`base64`、`dirs` の依存追加
- 設定ファイルなしでも `--out` と `--file` を指定すれば従来通り動作する（後方互換）
- viewer 側の設定ファイル (`ndp.config.json`) とは別ファイル（publisher 専用）
- ndp-server の body size limit がアートワークサイズによっては制約となる（別 issue で対応）
