# ADR-0025: 中継サーバ (ndp-server) のアーキテクチャ

## ステータス

Accepted

## コンテキスト

Phase 2 (クラウド中継サーバ) を実現するため、publisher と viewer をインターネット経由で中継するサーバが必要。以下の要件がある:

- publisher から楽曲情報を受信し、viewer にリアルタイムで転送する
- 複数の publisher が 1 つのセッションに参加できる（DJ リレー形式）
- viewer は iPad Safari 上の Web アプリとして動作する
- Phase 3 (LAN 内ポータブルデバイス) でも同じサーバコードを流用したい
- データ量は極めて小さい（JSON 数百バイト × 曲変更のたび）

### 検討した選択肢

#### 通信方式

| 選択肢 | メリット | デメリット |
|---|---|---|
| **REST + SSE (採用)** | publisher は POST で送信するだけ。viewer への push は SSE で一方向。シンプル | SSE は再接続時に一瞬途切れる |
| WebSocket (双方向) | 双方向通信。viewer → サーバ方向も使える | viewer → サーバの通信が不要。publisher 側も WebSocket 維持が必要になる |
| ポーリング | 最もシンプル | リアルタイム性が低い |

#### デプロイ先

| 選択肢 | メリット | デメリット |
|---|---|---|
| **Lightsail (採用)** | 月 $3.50。SSE 制限なし。シンプル | IAM ロールなし（アクセスキーで対応） |
| Lambda + API Gateway | スケールゼロ。使わない時コスト 0 | SSE の長時間接続に不向き（15 分タイムアウト） |
| ECS Fargate | CDK で管理可能。IAM ロール使用可 | 最小構成でも月 $10〜。オーバースペック |
| Fly.io / Railway | 無料枠あり | AWS エコシステムとの統合が弱い |

#### TLS 終端

| 選択肢 | メリット | デメリット |
|---|---|---|
| **Caddy (採用)** | Let's Encrypt 自動取得・更新。設定 1 行。SSE をデフォルトで正しく扱う | nginx より情報が少ない |
| nginx + certbot | 実績豊富 | 設定が冗長。certbot の cron 管理。SSE 用に `proxy_buffering off` 等が必要 |
| Cloudflare Proxy | 無料 TLS。DDoS 保護 | ネームサーバ移管が必要 |

#### Reverse HTTP

draft 段階（RFC 未成立）で実用的な実装がないため棄却。

#### CDK

Lightsail が CDK (CloudFormation) の管理対象外のため、現時点では導入しない。将来 DynamoDB/S3 等を追加する際に再検討する。

## 決定

### 技術スタック

- **言語**: Rust
- **フレームワーク**: axum + tokio
- **パッケージ名**: `ndp-server`
- **配置**: `apps/server/`

### API 設計

ベースパス: `/api/`

ヘルスチェック (`GET /health`) のみベースパスの外に配置する。

#### POST /api/sessions/create

viewer がセッションを開始する。

リクエスト:
```json
{
  "event_name": "DJ Night 2026-07-20"  // optional
}
```

レスポンス (201 Created):
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "code": "037482",
  "viewer_token": "vt_xxxxxxxxxxxx"
}
```

- `code`: 0 埋め 6 桁。publisher が join 時に入力する認証コード
- `viewer_token`: SSE 接続時の認証用

#### POST /api/sessions/join

publisher がセッションに参加する。

リクエスト:
```json
{
  "code": "037482",
  "dj_name": "DJ-A"
}
```

レスポンス (200 OK):
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "publisher_id": "pub_001",
  "token": "pt_xxxxxxxxxxxx"
}
```

- `token`: 以降の publish リクエストで `Authorization: Bearer pt_xxxxxxxxxxxx` として使用

#### POST /api/sessions/{session_id}/publish

publisher が楽曲情報を送信する。

ヘッダ:
```
Authorization: Bearer pt_xxxxxxxxxxxx
```

リクエスト:
```json
{
  "title": "Track Name",
  "artist": "Artist Name",
  "album": "Album Name",
  "comment": "House / 128BPM",
  "artwork": "data:image/jpeg;base64,/9j/4AAQ...",
  "updated_at": "2026-07-20T15:30:00+09:00"
}
```

- `album`, `comment`, `artwork` は optional
- `artwork` は Base64 Data URI (nullable)

レスポンス: 204 No Content

#### GET /api/sessions/{session_id}/stream

viewer が SSE で楽曲更新を受信する。

ヘッダ:
```
Authorization: Bearer vt_xxxxxxxxxxxx
```

SSE イベント:
```
event: track_changed
data: {"publisher_id":"pub_001","dj_name":"DJ-A","title":"Track Name","artist":"Artist Name","album":"Album Name","comment":"House / 128BPM","artwork":"data:image/jpeg;base64,...","updated_at":"2026-07-20T15:30:00+09:00"}

event: publisher_joined
data: {"publisher_id":"pub_001","dj_name":"DJ-A"}

event: heartbeat
data: {}
```

- 接続直後にサーバが最新の now_playing を即送信（途中接続対応）
- `heartbeat` はコネクション維持用（30 秒間隔）

### セッションモデル

- viewer がセッションを作成し、6 桁コードを画面に表示
- publisher がコードを入力して join → Bearer トークンを取得
- 1 セッションに複数 publisher + 1 viewer (SSE)
- publish を受けたら同セッションの SSE に転送
- セッションごとに最新の now_playing を保持（viewer 途中接続時に即送信）

### デプロイ構成

```
[Value Domain DNS]
  A relay.example.com → Lightsail Static IP

[Lightsail instance ($3.50/月)]
  Caddy (:443, auto TLS) → ndp-server (:8080)
```

- インフラ管理は AWS CLI ラッパースクリプトで行う
- デプロイはクロスコンパイル + scp + systemd

### ストレージ設計方針

初期はインメモリ（セッション = サーバプロセスのライフタイム）。将来的にセットリスト履歴等の永続化が必要になった際に DynamoDB/S3 を追加する。ストレージ層を trait で抽象化し、差し替え可能な設計とする。

## 影響

- `apps/server/` に新しい Rust プロジェクトが追加される
- publisher CLI に WebSocket/HTTP 送信機能を追加する必要がある（別 issue）
- Phase 2 用の Web 版 viewer を別途実装する必要がある（別 issue）
- Lightsail インスタンスの運用コスト ($3.50/月 + ドメイン代) が発生する
