# ADR-0038: viewer フロントエンドのブラウザ対応

## ステータス

Accepted

## コンテキスト

`apps/viewer` のフロントエンドは Tauri のデスクトップアプリとしてのみ動作する。#80 (ADR-0037) で、viewer が作成済みのセッションに view 権限で参加 (Connect) できるようになり、iPad などのブラウザで表示するという使い方が現実的になった (#77)。

web モードの通信は、すでに `fetch` と `EventSource`、画像は Base64 Data URI であり、ブラウザ API だけで動く。ブラウザで動かない原因は、`App.tsx` などに直接書かれた Tauri API の呼び出しである。たとえば `getCurrentWebviewWindow().label` はレンダリング時に呼ばれるため、ブラウザでは起動時に例外になり、白い画面になる。

## 決定

### 0. 実装の段階

本 ADR はブラウザ対応の全体の設計を定める。実装は次の段階に分ける。

| 段階 | 内容 |
|---|---|
| 第1段階 | `Platform` 抽象と Tauri 実装のみ。Tauri API の呼び出しを `platform/tauri.ts` に集約する。Tauri 版の動作は変えない |
| 第2段階以降 | ブラウザ実装 (`browser.ts`)、`capabilities`、`isTauri()` による実行時判定、設定 (`VITE_NDP_SERVER_URL`)、ビルドスクリプト (`build:web` など)、README |

第1段階の `Platform` には、`capabilities` と実行時判定を持たせない。ブラウザ実装が入る第2段階で追加する。以降の節は、第2段階までを含めた最終的な設計を記す。

### 1. ブラウザ版の位置づけ

ブラウザ版は web モードの Connect 専用とする。

| 機能 | ブラウザ版 |
|---|---|
| Connect (viewer 用コードで参加) | 対応 |
| Create (セッションの作成) | 非対応。ブラウザはセッションを作る側ではなく、タブを閉じてもセッションが残る問題を考えなくてよい |
| local モード | 非対応 (`mode` は `web` に固定) |
| `r` (設定再読み込み) | 無効。ショートカット一覧にも出さない |
| `c` / `t` / `e` (表示のトグル) | 対応。ステートのみで、永続化しない |
| `Shift+S` | セッションから退出 (従来の participant と同じ) |
| `b` (背景画像) | 非対応 (ローカルのファイルパスを扱えない)。背景の同期は #84 で扱う |
| `m` (モニタウィンドウ) | 非対応 |

### 2. プラットフォーム抽象 (`src/platform/`)

Tauri API への依存を、`Platform` インターフェースの裏側に閉じ込める。

- `types.ts`: `Platform` インターフェースと `capabilities`
- `tauri.ts`: 現行の動作 (Tauri API の呼び出し)
- `browser.ts`: ブラウザ向けの実装
- `index.ts`: `isTauri()` による実行時判定で、`platform` を 1 つ公開する

`Platform` が持つもの:

| メンバー | Tauri | ブラウザ |
|---|---|---|
| `capabilities` | すべて有効 | `createSession` / `configFile` / `monitorWindow` / `backgroundImage` がすべて無効 (local モードは、ブラウザ版の設定が `mode: "web"` で固定のため、`capabilities` では扱わない) |
| `windowLabel` | ウィンドウのラベル (`main` / `monitor`) | `main` |
| `getConfig()` / `reloadConfig()` | `get_app_config` / `reload_config` | ビルド時の値から作った固定の設定 (`reloadConfig` は未使用) |
| `saveSetting(key, value)` | `update_config_value` | 何もしない |
| `setWebSession()` / `clearWebSession()` | `set_web_session` / `clear_web_session` (終了時の破棄 / 退出) | `pagehide` の登録 / 解除 |
| `convertFileSrc(path)` | `convertFileSrc` | パスをそのまま返す (web モードの画像は Data URI) |
| `listBackgroundImages()` | `list_background_images` | 空配列 |
| `openMonitor()` / `sendTrackToMonitor()` / `listenMonitorTrack()` | モニタウィンドウの操作 | 何もしない |
| `startLocalDataSource()` | watcher の開始とイベントの購読 | 何もしない |

- 画面側は、`capabilities` を見て、ボタンの表示やショートカットを切り替える (Tauri かブラウザかを直接は判定しない)
- 実行時判定にするのは、`vite dev` で開いたブラウザでも同じコードを動かせるため。ビルドを 2 系統に分ける必要もない
- 両方の実装を静的に import する。`@tauri-apps/api` は import しただけでは何も実行せず、ブラウザでは呼ばれないため、動的 import にはしない (同期的な `windowLabel` と相性が悪く、バンドルサイズの差も小さい)

### 3. 設定の扱い (ブラウザ版)

- **サーバー URL**: ビルド時の環境変数 `VITE_NDP_SERVER_URL` で固定する
  - `.env.web` に記述する (`.gitignore` 対象)。ひな形として `.env.web.example` をコミットする
  - web ビルドでこの変数が未設定ならビルドを失敗させる。空文字は有効な値とする (同一オリジンで配信する場合の相対パス)
  - 末尾のスラッシュは取り除く
- **表示設定** (`enableComments` / `showTags` / `eventName` / `showEventName`): 画面のステートのみで管理する。初期値はコード内の既定値 (コメント非表示、タグ表示、イベント名なし、イベント名表示)
  - `saveSetting` は何もしない。将来 `localStorage` に対応する場合は、`Platform` の `saveSetting` / `getConfig` の実装を差し替えるだけで済む

### 4. セッションからの退出 (ブラウザ版)

- `Shift+S` では、従来どおり SSE を切断して `POST /disconnect` を送る
- タブを閉じる、リロードするなどで離れるときは、`pagehide` で `fetch(..., { keepalive: true })` により `POST /disconnect` を送る (ベストエフォート)
  - `Authorization` ヘッダ付きのリクエストのため、CORS のプリフライトが発生する。ブラウザによっては `keepalive` と併用できず、リクエストが届かないことがある
  - 届かなくても、参加者用 token が残るだけで、閲覧権限以外の影響はない。token の失効の自動化は #81 で扱う

### 5. ビルドとスクリプト

- `vite.config.ts` を関数形式 (`defineConfig(({ mode }) => ...)`) に変更し、`mode === "web"` のとき `build.outDir` を `dist-web` にする。従来の `yarn build` は `dist` のまま
- `package.json` に追加するスクリプト:
  - `build:web`: `tsc -b && vite build --mode web`
  - `dev:web`: `vite --mode web --port 1421` (Tauri の開発サーバー 1420 と同時に起動できる)
  - `preview:web`: `vite preview --mode web`
- `.gitignore` に `dist-web/` と `.env.web` を追加する

### 6. 配信

本 ADR では、成果物 (`dist-web/`) を出力するところまでを扱う。配信方法は [ADR-0039](0039-20261103-viewer-web-deploy.md) で決めた (ndp-server と同じホストの `/v/` で配信する)。ビルドは、配信パスに依存しないよう、相対パスで出力する (当初の「ルートパス前提」は、これに置き換えた)。

## 理由

- **Connect 専用**: ブラウザ版の主な用途は、iPad などでの閲覧。セッションを作る側にすると、タブを閉じたときのセッション破棄 (`pagehide` や TTL) という別の論点が増える
- **実行時判定**: ビルドを分けると、`vite dev` のブラウザ確認ができない。1 つのコードを両環境で動かすほうが、保守が単純
- **capabilities で画面を切り替える**: 画面側に「ブラウザか Tauri か」の分岐を散らさず、機能の有無で判断できる
- **ステートのみの設定**: ブラウザには設定ファイルがなく、必要なのはサーバー URL だけ。表示設定は、Connect 専用なら毎回の既定値で足りる

## 影響

- `App.tsx`、`useDataSource.ts`、`artwork.ts`、`BackgroundPicker.tsx` から、Tauri API の直接呼び出しがなくなる (`platform/tauri.ts` に集約)
- Tauri 版の動作は変えない (Create / Connect、local モード、モニタウィンドウ、背景画像、設定の再読み込み)
- ブラウザ版のバンドルには、Tauri API のモジュールが含まれる (呼び出されないため動作には影響しない)
- ブラウザ版では、表示設定がリロードで初期値に戻る
- viewer token が SSE の URL クエリに載る (`EventSource` はヘッダを付けられない) ため、ブラウザ配信時のアクセスログに残る。現状の Tauri 版と同じ扱いで、参加者用 token は購読のみの権限
