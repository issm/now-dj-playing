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
| POST | `/api/sessions/join` | セッション参加 (publisher 用) | なし |
| DELETE | `/api/sessions/{id}` | セッション破棄 (viewer 用) | Bearer トークン (viewer_token) |
| POST | `/api/sessions/{id}/publish` | 楽曲情報の送信 | Bearer トークン |
| POST | `/api/sessions/{id}/leave` | セッション離脱 | Bearer トークン |
| GET | `/api/sessions/{id}/stream` | SSE ストリーム (viewer 用) | Bearer トークン |

### SSE イベント

| イベント名 | トリガー | データ |
|---|---|---|
| `track_changed` | publisher が楽曲情報を publish | `{ publisher_id, dj_name, title, artist, album?, comment?, artwork?, updated_at }` |
| `publisher_joined` | publisher がセッションに join | `{ publisher_id, dj_name, dj_image? }` |
| `publisher_left` | publisher がセッションから leave | `{ publisher_id, dj_name }` |
| `heartbeat` | コネクション維持 (30 秒間隔) | — |

### フロー

1. viewer が `POST /api/sessions/create` でセッションを作成 → 6桁コードと viewer_token を取得
2. publisher が `POST /api/sessions/join` に6桁コードを送信 → publisher 用トークンを取得
3. publisher が `POST /api/sessions/{id}/publish` で楽曲情報を送信 (Bearer トークン必須)
4. viewer が `GET /api/sessions/{id}/stream` で SSE 接続 → イベントを受信
5. publisher が `POST /api/sessions/{id}/leave` で離脱 → viewer に `publisher_left` が配信
6. viewer が `DELETE /api/sessions/{id}` でセッションを破棄 (アプリ終了時等)

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
