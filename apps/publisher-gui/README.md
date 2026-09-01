# ndp-publish-gui

now-dj-playing の Publisher GUI アプリ。楽曲ファイルをドラッグ&ドロップで publish できるデスクトップアプリケーション。

## 技術スタック

- Tauri 2 (macOS)
- React + TypeScript
- Vite
- Tailwind CSS v4
- ndp_publish (バックエンドはライブラリクレートを共有)

## 機能

- 楽曲ファイル (mp3/m4a) のドラッグ&ドロップによる publish
- セッション join / leave
- local モード / web モードの切り替え
- 「基本」タブでの共通設定管理（DJ 名テキスト、DJ 画像）
- DJ 画像のドラッグ&ドロップ設定・プレビュー表示
- join 時に DJ 画像を 800x800 リサイズして送信
- アプリ終了時の自動 leave (ベストエフォート)
- always-on-top ウィンドウ (320x420)
- アプリ隣接の設定ファイル自動検出
- 設定ファイルの GUI 編集・保存（`base` 空間形式）

## セットアップ

```bash
cd apps/publisher-gui
yarn install
```

## 開発

```bash
cargo tauri dev
```

`http://localhost:1421` で Vite 開発サーバが起動し、Tauri ウィンドウが表示される。

## 設定ファイル

publisher CLI と同じ形式の `ndp-publish.config.json` を使用する。

### ルックアップ順

1. アプリ隣接ディレクトリの `ndp-publish.config.json` (macOS .app バンドルの場合は `.app` の親ディレクトリ)
2. publisher CLI と同じルックアップ (環境変数 → CWD → `~/.config/ndp/`)

設定ファイルが見つからない場合、GUI から設定を入力してアプリ隣接に新規作成できる。

## ビルド

```bash
cargo tauri build
```

出力: `src-tauri/target/release/bundle/macos/ndp-publish-gui.app`

## 関連

- [apps/publisher](../publisher/) — CLI 版 publisher (ライブラリクレートを共有)
- [ADR-0029](../../docs/adr/0029-20260723-publisher-gui.md) — Publisher GUI アプリの設計
- [ADR-0034](../../docs/adr/0034-20260726-dj-image-web-flow.md) — DJ 画像の全レイヤー対応

## アプリアイコン

元画像は `assets/app-icon-master.png`（1254x1254 / 透過 PNG / squircle デザイン）。
アイコン一式は次のスクリプトで再生成する。

```bash
./scripts/build-icons.sh
```

処理内容:

1. 元画像を 824x824 に縮小し、1024x1024 の透過キャンバス中央に配置（macOS のアイコン余白に合わせる）
2. `cargo tauri icon` で `src-tauri/icons/` 配下のアイコン一式を生成
3. macOS 専用のため、Android 向けと未参照の `64x64.png` を破棄

必要なコマンド: `magick` (ImageMagick 7), `cargo-tauri`

`tauri.conf.json` の `bundle.icon` は `32x32.png` / `128x128.png` / `128x128@2x.png` / `icon.icns` / `icon.ico` を参照する。
