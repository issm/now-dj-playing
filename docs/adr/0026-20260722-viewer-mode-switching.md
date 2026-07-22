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
- 後方互換のフォールバックは設けない（開発序盤のため不要と判断）

### フロントエンドのライフサイクル

```
App 起動
  → get_app_config (設定読み込み)
  → appConfig state にセット
  → useDataSource フックが appConfig の変化を検知してデータソースを開始:
      local: start_watch (Tauri IPC で track-changed イベントを listen)
      web:   セッション作成 → SSE 接続 (EventSource で track_changed を受信)
  → track 情報を受信したら UI 更新
```

設定再読み込み時 (`r` キー) も同様に `appConfig` state を更新するだけで、`useDataSource` の `useEffect` が古いデータソースを cleanup し新しいデータソースを起動する。`handleReloadConfig` はモードを意識しない。

### データソースの抽象化: useDataSource フック

`apps/viewer/src/useDataSource.ts` にモード切り替えロジックを集約する。

```ts
useDataSource(appConfig, {
  onTrack: (track) => { ... },
  onError: (message) => { ... },
  onSessionCreated: (code) => { ... },  // web モードのみ
});
```

- `local` モード: `listen("track-changed")` + `invoke("start_watch")`
- `web` モード: `fetch` でセッション作成 → `EventSource` で SSE 受信

どちらのモードでも `TrackPayload` 型に変換してコールバックに渡すため、UI コンポーネント側はモードを意識しない。

### web モードの動作

1. `get_app_config` で設定を読み込み、`mode === "web"` を検出
2. `server_url` を使って `POST /api/sessions/create` を呼ぶ
3. 返された6桁コードを `onSessionCreated` 経由で通知し、待機画面に `(xxxxxx)` として表示する（publisher が join するために必要）
4. `viewer_token` を使って `GET /api/sessions/{id}/stream?token=...` に SSE 接続
5. `track_changed` イベントを受信したら `TrackPayload` に変換して表示

### SSE 認証方式

`EventSource` API は任意の HTTP ヘッダを送信できないため、`Authorization` ヘッダではなく **クエリパラメータ `?token=...`** でトークンを渡す。サーバ側 (ndp-server) はクエリパラメータと `Authorization` ヘッダの両方をサポートする（ADR-0025 参照）。

### CORS

Tauri の WebView から `http://localhost:8080` への `fetch` / `EventSource` は別オリジンとして扱われるため、ndp-server 側で CORS を許可する必要がある（ADR-0025 参照）。

### アートワークの表示方式

`TrackPayload.artworkPath` / `djLogoPath` は local モードではローカルファイルパス、web モードでは ndp-server から受け取る Base64 Data URI (`data:image/...`) のいずれかが入る。`apps/viewer/src/artwork.ts` の `resolveImageSrc()` で両者を判別する:

- `data:` から始まる場合 → そのまま `<img src>` に使う（Data URI はコンテンツが変わればデータ自体が変わるため、キャッシュバスターは不要）
- それ以外 → ローカルファイルパスとして `convertFileSrc()` で変換し、`updatedAt` によるキャッシュバスターを付与する

`useDataSource.ts` の web モード側では、ndp-server から受け取った `TrackData.artwork` をそのまま `TrackPayload.artworkPath` に格納する（DJ ロゴは ndp-server の TrackData に対応するフィールドがないため未対応）。

### 未対応・将来対応

- **DJ ロゴ**: ndp-server の `TrackData` に DJ ロゴ用フィールドがないため、web モードでは非対応
- **背景画像**: web モードでもローカルファイルシステムの背景画像設定は有効（Tauri アプリとして動作するため `convertFileSrc` が使える）。iPad Safari 上の Web viewer（別 issue）では利用不可

## 影響

- 設定ファイルの構造が変わる（後方互換なし、development.json を手動更新）
- Rust 側の設定パーサー (`config.rs`) を更新
- フロントエンドに `useDataSource` フックを追加、`App.tsx` のデータ取得ロジックを移行
- ndp-server 側に CORS ミドルウェアとクエリパラメータ認証を追加（ADR-0025 に記載）
- web モード時の待機画面にセッションコードを表示するようになった
- web モードでアートワーク表示に対応（Base64 Data URI）
