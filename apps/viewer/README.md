# ndp-viewer

now-dj-playing の表示アプリ。DJ プレイ中の楽曲情報（曲名・アーティスト・アルバム・アートワーク）をリアルタイムに表示する macOS デスクトップアプリ。

## 技術スタック

- Tauri 2 (macOS)
- React + TypeScript
- Vite
- Tailwind CSS v4
- watch-core (ファイル監視、local モード)

## 動作モード

### Local モード

`watch-core` による `.ready` ファイル監視で楽曲情報を取得する。publisher が共有ディレクトリに書き出したファイルを監視し、変更を検知して表示を更新する。

### Web モード

ndp-server に SSE 接続して楽曲情報をリアルタイム受信する。複数 DJ のロースター表示に対応。起動後、次のどちらかでセッションに接続する。

| 操作 | 説明 |
|---|---|
| Create | セッションを新規作成する。画面に表示される 2 つのコードを共有する。`publish` は publisher 用、`view` は閲覧のみの viewer 用 |
| Connect | 作成済みのセッションに、`view` のコード（6 桁）を入力して参加する。閲覧のみで、複数の viewer が同時に表示できる |

- Create した viewer に表示されるコードは、クリックするとクリップボードにコピーされる（ラベルとコードは、マウスでは選択できない）
- 設定再読み込み (`r`) 時はセッションを維持する（`serverUrl` 変更時のみセッションを終了する）
- Create した viewer: `Shift+S` またはアプリ終了時に、サーバー側のセッションを破棄する
- Connect した viewer: `Shift+S` またはアプリ終了時に、セッションから退出する（セッションは維持される）。Create した viewer がセッションを破棄すると、「セッションが終了しました」と表示されて待機画面に戻る
- Connect には ndp-server 0.4.0 以降が必要

## ブラウザ版

Tauri なしで、ブラウザで動かせる。web モードの **Connect 専用**で、iPad などでの閲覧を想定している。設計は [ADR-0038](../../docs/adr/0038-20261103-viewer-browser-support.md) を参照。

| 機能 | ブラウザ版 |
|---|---|
| Connect（`view` のコードで参加） | 対応 |
| 表示のトグル（`c` `t` `e`） | 対応（永続化しない。リロードで初期値に戻る） |
| 退出（`Shift+S`） | 対応。ページを閉じる / リロードするときも、退出を試みる（ベストエフォート） |
| Create、local モード | 非対応 |
| 設定ファイル（`r`）、背景画像（`b`）、モニタウィンドウ（`m`） | 非対応（ショートカット一覧にも出ない） |

サーバー URL は、ビルド時の環境変数 `VITE_NDP_SERVER_URL` で固定する（バンドルに埋め込まれる）。未設定だとビルドが失敗する。同一オリジンで配信する場合は、空にする。

```bash
cd apps/viewer
cp .env.web.example .env.web   # VITE_NDP_SERVER_URL を編集する

yarn dev:web      # 開発サーバー (http://localhost:1421)
yarn build:web    # dist-web/ に出力する（Tauri 用の dist/ とは別）
yarn preview:web  # dist-web/ を確認する (http://localhost:4173)
```

`dist-web/` は静的ファイルなので、任意の静的ホスティングで配信できる。ルートパス（`/`）での配信を前提とする。ndp-server は CORS を全オリジンに許可している。配信方法の取り決めは、この README では扱わない。

## セットアップ

```bash
cd apps/viewer
yarn install
```

## 開発

```bash
# 設定ファイルを作成
cp src-tauri/config/development.json.example src-tauri/config/development.json
# development.json を環境に合わせて編集

# 起動
cargo tauri dev
```

設定ファイルは `.envrc` の `NDP_CONFIG` 環境変数で指定されたパスから読み込まれる。

## 設定ファイル (`ndp.config.json`)

形式: JSONC（`//` コメント、trailing comma を許容）

```jsonc
{
  "mode": "local",  // "local" | "web"
  // local モード設定
  "local": {
    "watch_dir": "~/ndp",
    "dj_id": "dj-000"
  },
  // web モード設定
  "web": {
    "endpoint_url": "https://relay.example.com"
  },
  "event_name": "Club Night vol.3",
  "enable_comments": true,
  "show_tags": true,
  "background_image": {
    "base_dir": "~/apps/ndp/bg",
    "path": "background.png"
  }
}
```

| キー | デフォルト値 | 説明 |
|---|---|---|
| `mode` | `"local"` | データソースモード |
| `local.watch_dir` | `"~/ndp"` | 監視対象ベースディレクトリ |
| `local.dj_id` | `"dj-000"` | 対象 DJ ディレクトリ名 |
| `web.endpoint_url` | - | ndp-server の API エンドポイント URL |
| `event_name` | *(なし)* | イベント名の表示テキスト |
| `show_event_name` | `true` | イベント名の表示/非表示 |
| `enable_comments` | `false` | コメント表示の初期状態 |
| `show_tags` | `true` | タグ表示の初期状態 |
| `background_image.base_dir` | - | 背景画像格納ディレクトリ |
| `background_image.path` | `null` | 背景画像パス (`null` で「なし」) |

パス指定: 絶対パス / `~` 付き / 相対パス（設定ファイル基準）に対応。

## キーボードショートカット

| キー | 説明 |
|---|---|
| `r` | 設定ファイルの再読み込み |
| `b` | 背景画像の選択 |
| `c` | コメント表示のトグル |
| `t` | タグ表示のトグル |
| `e` | イベント名表示のトグル |
| `Shift + s` | セッションの破棄（Create した場合）/ 退出（Connect した場合） |
| `m` | モニタウィンドウを開く |
| `?` | ショートカット一覧 |
| `Escape` | オーバーレイを閉じる |

Connect 用のコード入力欄にフォーカスがあるときは、ショートカットは無効になる。

## モニタウィンドウ

`m` キーでコンパクトなモニタウィンドウ (240x280, always-on-top) を起動できる。メインウィンドウのトラック情報が同期表示される。

## ビルド

```bash
cargo tauri build
```

出力: `src-tauri/target/release/bundle/macos/now-dj-playing.app`

## アプリアイコン

元画像は `assets/app-icon-master.png`（1254x1254 / 透過 PNG / 円形デザイン）。
アイコン一式は次のスクリプトで再生成する。

```bash
./scripts/build-icons.sh
```

処理内容:

1. 元画像を直径 824px に縮小し、1024x1024 の透過キャンバス中央に配置（macOS のアイコン余白に合わせる）
2. `cargo tauri icon` で `src-tauri/icons/` 配下のアイコン一式を生成
3. macOS 専用のため、Android 向けと未参照の `64x64.png` を破棄

必要なコマンド: `magick` (ImageMagick 7), `cargo-tauri`

`tauri.conf.json` の `bundle.icon` は `32x32.png` / `128x128.png` / `128x128@2x.png` / `icon.icns` / `icon.ico` を参照する。
