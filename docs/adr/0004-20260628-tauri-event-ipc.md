# ADR-0004: Tauri Event による フロント・バックエンド連携

## ステータス

Accepted

## コンテキスト

フロントエンド (React) とバックエンド (Rust watcher) 間でリアルタイムに楽曲情報を伝達する必要がある。WebSocket のようなプッシュ型の通信が求められる。

## 決定

### Tauri Event システムを使用する

WebSocket ではなく、Tauri の IPC Event を採用する。

- **`emit("track-changed", payload)`** — Rust 側から楽曲情報を push
- **`listen("track-changed", callback)`** — React 側でイベントを購読
- **`invoke("start_watch", { baseDir })`** — React 側から監視開始を指示

### イベント一覧

| イベント名 | 方向 | ペイロード |
|---|---|---|
| `track-changed` | Rust → React | `TrackPayload` (楽曲情報) |
| `dj-removed` | Rust → React | DJ ディレクトリ名 |
| `watch-error` | Rust → React | `ErrorPayload` (エラー情報) |

### 起動時の既存データスキャン

監視開始時にベースディレクトリ内の既存 `.ready` を走査し、有効なデータがあれば即座に `track-changed` を emit する。これにより、アプリ起動時に既に書き出されている楽曲情報を待機なく表示できる。

### 監視ディレクトリの指定

- `VITE_WATCH_DIR` 環境変数で指定する
- 開発時は `apps/viewer/.env.development` に記載
- 未設定時はエラーメッセージを表示して監視を開始しない

## 理由

- Tauri Event は同一プロセス内の IPC であり、WebSocket に比べて接続管理やネットワークスタックが不要
- serde による自動シリアライズでボイラープレートが少ない
- リアルタイム push が可能で、WebSocket と同等のユースケースをカバーする

## 影響

- フロントエンドは `@tauri-apps/api` への依存が必要
- Tauri 外（ブラウザ単体等）での動作はできない
