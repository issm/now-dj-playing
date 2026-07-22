# ADR-0026: viewer のデータソースモード切り替え

## ステータス

Accepted

## コンテキスト

Phase 2 で ndp-server を導入するにあたり、viewer が楽曲情報を受信する方法を2つサポートする必要がある:

1. **local**: 従来通り、ローカルのディレクトリ監視 (Tauri IPC 経由)
2. **web**: ndp-server に SSE 接続して楽曲情報を受信

viewer は引き続き Tauri アプリとして動作し、設定ファイルの `mode` でデータソースを切り替える。

## 決定

### 設定ファイル構造

```jsonc
{
  "mode": "local",  // "local" | "web"
  "local": {
    "watch_dir": "/tmp/ndp",
    "dj_id": "dj-000"
  },
  "web": {
    "server_url": "http://localhost:8080"
  },
  // 共通（表示系）
  "event_name": "Club Night vol.3",
  "show_event_name": true,
  "enable_comments": true,
  "show_tags": true,
  "background_image": {
    "base_dir": "~/apps/ndp/bg",
    "path": "background.png"
  }
}
```

- `mode`: データソースの選択。デフォルトは `"local"`（省略時も local として扱う）
- `local`: ローカルモード固有の設定（`watch_dir`, `dj_id`）
- `web`: Web モード固有の設定（`server_url` 等）
- トップレベルの表示系設定は両モード共通

### フロントエンドのライフサイクル

```
App 起動
  → get_app_config (設定読み込み)
  → mode に応じてデータソースを開始:
      local: start_watch (Tauri IPC で track-changed イベントを listen)
      web:   セッション作成 → SSE 接続 (EventSource で track_changed を受信)
  → track 情報を受信したら UI 更新
```

フロントエンドから見ると、データソースの開始タイミングと受信するデータの形状が統一される。

### データソースの抽象化

フロントエンド側で「データソース」の概念を導入し、モードに応じた実装を切り替える:

- `local` モード: Tauri IPC (`listen("track-changed")`)
- `web` モード: `EventSource` API で ndp-server の SSE に接続

どちらのモードでも `TrackPayload` 相当のデータを UI コンポーネントに渡す。

### web モードの動作

1. `get_app_config` で設定を読み込み、`mode === "web"` を検出
2. `server_url` を使って `POST /api/sessions/create` を呼ぶ
3. 返されたセッションコードを画面に表示（publisher が join するために必要）
4. `GET /api/sessions/{id}/stream` に SSE 接続
5. `track_changed` イベントを受信したら `TrackPayload` に変換して表示

### 既存設定との互換性

- `mode` を省略した場合は `"local"` として動作
- `local` セクションを省略した場合は、トップレベルの `watch_dir` / `dj_id` をフォールバックとして使用（移行期間中）
- 将来的にはフォールバックを廃止し、`local` セクション必須とする

## 影響

- 設定ファイルの構造が変わる（後方互換あり）
- Rust 側の設定パーサー (`config.rs`) を更新する必要がある
- フロントエンドに `EventSource` ベースのデータ受信ロジックを追加
- `web` モード時のセッション管理 UI（コード表示等）が必要
