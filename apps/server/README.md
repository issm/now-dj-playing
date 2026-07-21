# ndp-server

now-dj-playing の中継サーバ。publisher から楽曲情報を受信し、SSE (Server-Sent Events) で viewer にリアルタイム配信する。

## 技術スタック

- Rust (axum + tokio)
- セッション管理: インメモリ
- リアルタイム配信: SSE (tokio broadcast channel)

## API

ベースパス: `/api/`

| メソッド | パス | 用途 | 認証 |
|---|---|---|---|
| GET | `/health` | ヘルスチェック | なし |
| POST | `/api/sessions/create` | セッション作成 (viewer 用) | なし |
| POST | `/api/sessions/join` | セッション参加 (publisher 用) | なし |
| POST | `/api/publish` | 楽曲情報の送信 | Bearer トークン |
| GET | `/api/sessions/{id}/stream` | SSE ストリーム (viewer 用) | Bearer トークン |

### フロー

1. viewer が `POST /api/sessions/create` でセッションを作成 → 6桁コードと viewer_token を取得
2. publisher が `POST /api/sessions/join` に6桁コードを送信 → publisher 用トークンを取得
3. publisher が `POST /api/publish` で楽曲情報を送信 (Bearer トークン必須)
4. viewer が `GET /api/sessions/{id}/stream` で SSE 接続 → `track_changed` イベントを受信

## 開発

### 起動

```bash
cd apps/server
cargo run
```

`http://localhost:8080` で起動します。

### テスト

サーバ起動後:

```bash
bash test.sh
```

別のホスト・ポートに対して実行する場合:

```bash
bash test.sh http://localhost:8081
```

### Docker (ローカルテスト用)

```bash
# ビルド
docker build -t ndp-server .

# 実行
docker run --rm -p 8080:8080 ndp-server
```

## デプロイ

本番環境は Lightsail + Caddy (TLS 自動) を想定。クロスコンパイルしたバイナリを直接配置する。

```
[Caddy :443] → [ndp-server :8080 (systemd)]
```

```bash
# ビルド (macOS → Linux)
cross build --release --target x86_64-unknown-linux-gnu -p ndp-server

# 配置
scp target/x86_64-unknown-linux-gnu/release/ndp-server user@server:/opt/ndp/
ssh user@server "sudo systemctl restart ndp-server"
```

詳細は `docs/adr/0025-20260720-ndp-server-architecture.md` を参照。
