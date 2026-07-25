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

ndp-server に SSE 接続して楽曲情報をリアルタイム受信する。セッション作成 → 6桁コードを publisher に共有 → 楽曲情報を受信、の流れで動作する。複数 DJ のロースター表示に対応。

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
| `m` | モニタウィンドウを開く |
| `?` | ショートカット一覧 |
| `Escape` | オーバーレイを閉じる |

## モニタウィンドウ

`m` キーでコンパクトなモニタウィンドウ (240x280, always-on-top) を起動できる。メインウィンドウのトラック情報が同期表示される。

## ビルド

```bash
cargo tauri build
```

出力: `src-tauri/target/release/bundle/macos/now-dj-playing.app`
