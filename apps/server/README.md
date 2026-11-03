# ndp-server

now-dj-playing の中継サーバ。publisher から楽曲情報を受信し、SSE (Server-Sent Events) で viewer にリアルタイム配信する。

## 技術スタック

- Rust (axum + tokio)
- セッション管理: インメモリ
- リアルタイム配信: SSE (tokio broadcast channel)
- ログ: JSON 構造化ログ (tracing-subscriber)

## API

ベースパス: `/api/`

| メソッド | パス | 用途 | 認証 |
|---|---|---|---|
| GET | `/health` | ヘルスチェック | なし |
| POST | `/api/sessions/create` | セッション作成 (viewer 用) | なし |
| POST | `/api/sessions/join` | セッション参加 (publisher 用) | なし (publish 用コード) |
| POST | `/api/sessions/connect` | セッション参加 (閲覧専用の viewer 用) | なし (view 用コード) |
| DELETE | `/api/sessions/{id}` | セッション破棄 (作成した viewer のみ) | Bearer トークン (owner の token: `create` の `viewer_token`) |
| POST | `/api/sessions/{id}/publish` | 楽曲情報の送信 | Bearer トークン (publisher token) |
| POST | `/api/sessions/{id}/leave` | セッション離脱 (publisher 用) | Bearer トークン (publisher token) |
| POST | `/api/sessions/{id}/disconnect` | セッション離脱 (`connect` した viewer 用) | Bearer トークン (参加者用 token) |
| GET | `/api/sessions/{id}/stream` | SSE ストリーム (viewer 用) | Bearer トークン (owner の token または参加者用 token) |

### 権限モデル

| 権限 | できること | 渡し方 | コード |
|---|---|---|---|
| owner | 購読、セッション破棄 | `create` のレスポンスで token を受け取る | 不要 |
| publish | 楽曲の送信、`leave` | publish 用コードで `join` | `roles.publisher.code` (= `code`) |
| view | 購読、`disconnect` | view 用コードで `connect` | `roles.viewer.code` |

- publish 用と view 用のコードは別の値。互いのエンドポイントに入れると 404 になる
- 参加者用 token は `connect` の呼び出しごとに発行され、購読のみできる。`DELETE` は 403
- 詳細は [ADR-0036](../../docs/adr/0036-20261103-server-viewer-connect.md) を参照

### `POST /api/sessions/create` のレスポンス

```json
{
  "session_id": "<UUID v4>",
  "code": "037482",
  "event_name": null,
  "viewer_token": "vt_<UUID>",
  "roles": {
    "publisher": { "code": "037482" },
    "viewer":    { "code": "482910" }
  }
}
```

- `roles.publisher.code` はトップレベルの `code` と同じ値。トップレベルの `code` と `viewer_token` は過去互換のために残している
- トップレベルの `viewer_token` は **owner の token** (購読と破棄ができる)。`roles.viewer` とは無関係

### `POST /api/sessions/connect`

- リクエスト: `{ "code": "<view 用コード>" }`
- レスポンス: `{ "session_id", "viewer_token", "event_name" }`。`viewer_token` は参加者用 token (購読のみ)
- 無効なコードは 404

### `POST /api/sessions/{id}/disconnect`

- 参加者用 token を失効させる。成功時は 204。以降その token での購読は 401
- Authorization なし、または参加者用 token として登録されていない値 (owner の token を含む) は 401、セッションなしは 404

### SSE イベント

| イベント名 | トリガー | データ |
|---|---|---|
| `track_changed` | publisher が楽曲情報を publish | `{ publisher_id, dj_name, title, artist, album?, comment?, artwork?, updated_at }` |
| `publisher_joined` | publisher がセッションに join | `{ publisher_id, dj_name, dj_image? }` |
| `publisher_left` | publisher がセッションから leave | `{ publisher_id, dj_name }` |
| `session_destroyed` | owner がセッションを破棄 (破棄の直前) | `{ type: "session_destroyed" }` |
| `heartbeat` | コネクション維持 (30 秒間隔) | — |

SSE の接続直後に、参加中の publisher 全員分の `publisher_joined` と、最新の `track_changed` が順に送られる (途中参加・再接続でもロスターと楽曲が復元される)。参加の瞬間と重なると `publisher_joined` が重複して届くことがあるため、受信側は冪等に扱う。

### フロー

1. viewer が `POST /api/sessions/create` でセッションを作成 → publish 用コード、view 用コード、owner の token を取得
2. publisher が `POST /api/sessions/join` に publish 用コードを送信 → publisher 用トークンを取得
3. publisher が `POST /api/sessions/{id}/publish` で楽曲情報を送信 (Bearer トークン必須)
4. viewer が `GET /api/sessions/{id}/stream` で SSE 接続 → イベントを受信
5. 別の viewer が `POST /api/sessions/connect` に view 用コードを送信 → 参加者用 token を取得し、同じく SSE で購読
6. publisher が `POST /api/sessions/{id}/leave` で離脱 → viewer に `publisher_left` が配信
7. 参加者用 token の viewer が `POST /api/sessions/{id}/disconnect` で離脱
8. owner の viewer が `DELETE /api/sessions/{id}` でセッションを破棄 (アプリ終了時等) → 購読中の viewer に `session_destroyed` が配信

## 開発

### 起動

```bash
cd apps/server
cargo run
```

`http://localhost:8080` で起動します。

ログは JSON 形式で出力されるため、開発時は `jq` でパイプすると読みやすい:

```bash
cargo run 2>&1 | jq .
```

`RUST_LOG` 環境変数でフィルタレベルを制御可能 (デフォルト: `info,tower_http=debug`)。

### テスト

サーバ起動後:

```bash
bash test.sh
```

別のホスト・ポートに対して実行する場合:

```bash
bash test.sh http://localhost:8081
```

## デプロイ

本番環境は Lightsail + Caddy (TLS 自動) を想定。クロスコンパイルしたバイナリを直接配置する。

```
[Caddy :443] → [ndp-server :8080 (systemd)]
```

### ビルド (macOS → Linux x86_64)

`cargo-zigbuild` + musl ターゲットで静的リンクバイナリを生成する。

```bash
# 前提: brew install zig && cargo install cargo-zigbuild
# ターゲット追加 (初回のみ)
rustup target add x86_64-unknown-linux-musl

# ビルド
cargo zigbuild --release --target x86_64-unknown-linux-musl -p ndp-server
```

出力: `target/x86_64-unknown-linux-musl/release/ndp-server` (静的リンク, ~1.6MB)

### 配置

```bash
scp target/x86_64-unknown-linux-musl/release/ndp-server user@server:/opt/ndp/
ssh user@server "sudo systemctl restart ndp-server"
```

詳細は `docs/adr/0025-20260720-ndp-server-architecture.md` を参照。
