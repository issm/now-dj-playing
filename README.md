# now-dj-playing

DJプレイ中の楽曲情報（曲名・アーティスト・アルバム・アートワーク）をリアルタイムに表示するアプリケーション。

## 概要

DJプレイ中の楽曲情報をリアルタイムに iPad へ表示するシステム。

現在は Mac 上で publisher（楽曲情報の抽出）と viewer（表示）を動作させ、iPad を Sidecar でセカンダリモニタとして利用する構成で運用している。

```mermaid
graph LR
    subgraph Mac
        File[楽曲ファイル<br/>mp3/m4a]
        P[Publisher]
        Dir[共有ディレクトリ]
        V[Viewer ウィンドウ]
    end

    File -->|タグ抽出| P
    P -->|書き出し| Dir
    Dir -->|ファイル監視| V
    V -->|Sidecar<br/>P2P Wi-Fi| iPad[iPad<br/>セカンダリモニタ]
```

将来的には複数 publisher 対応や中継サーバ経由での構成を予定。詳細は [docs/vision.md](docs/vision.md) を参照。

## プロジェクト構成

```
now-dj-playing/
├─ apps/
│   ├─ viewer/          Tauri 2 + React + Vite + Tailwind (macOS 表示アプリ)
│   └─ publisher/       楽曲タグ抽出 CLI ツール (Rust)
├─ packages/
│   ├─ watch-core/      ファイル監視コアロジック (pure Rust)
│   └─ shared/schemas/  JSON スキーマ定義
├─ sandbox/             開発用ダミーデータ
└─ docs/adr/            Architecture Decision Records
```

## セットアップ

### 前提条件

- Node.js (nodenv で管理、バージョンは `.envrc` 参照)
- Rust (rustup)
- Yarn v1
- Xcode (Tauri macOS ビルド時)

### 手順

```sh
# direnv を有効化
direnv allow

# viewer の依存インストール
cd apps/viewer
yarn install

# publisher のビルド
cd apps/publisher
cargo build
```

## 開発

### Viewer (表示アプリ)

```sh
cd apps/viewer

# .env.development を作成 (.env.example を参照)
cp .env.example .env.development
# VITE_WATCH_DIR と VITE_DEFAULT_DJ_ID を設定

# 開発サーバー起動
cargo tauri dev
```

`VITE_WATCH_DIR` はベースディレクトリを指定する。viewer は起動時にこのディレクトリを自動作成し、再帰的に監視する。`VITE_DEFAULT_DJ_ID` で指定した DJ ディレクトリが後から作成されても検知する（起動順序に依存しない）。

### Publisher (書き出しツール)

```sh
cd apps/publisher

cargo run -- --file /path/to/track.mp3 --out /path/to/shared/ --id dj-000 --dj-name "DJ名"
```

#### オプション

| オプション | 必須 | デフォルト | 説明 |
|---|---|---|---|
| `--file` | ✓ | - | 楽曲ファイルパス (mp3, m4a) |
| `--out` | ✓ | - | 出力先ベースディレクトリ |
| `--id` | - | `dj-000` | DJ ディレクトリ名 |
| `--dj-name` | - | - | DJ 名テキスト or ロゴ画像パス |

### Watch Core (検証)

```sh
cd packages/watch-core

# ローカル監視の動作確認
cargo run --example watch_local -- ../../sandbox
```

## データプロトコル

### ディレクトリ構成

```
{base_dir}/
└─ {dj-id}/
    ├─ dj-profile[.txt|.png|.jpg|.jpeg]  DJ プロファイル
    ├─ now_playing.json                   楽曲情報
    ├─ artwork.{png|jpg|jpeg}             アートワーク (optional)
    └─ .ready                             同期完了マニフェスト
```

### .ready (マニフェスト)

全ファイル書き出し後に最後に作成される。viewer はこのファイルの変更のみを検知する。

```json
{
  "updated_at": "2026-06-28T15:30:00+09:00",
  "files": ["now_playing.json", "artwork.png"]
}
```

### now_playing.json

```json
{
  "title": "曲名",
  "artist": "アーティスト名",
  "album": "アルバム名",
  "artwork": "artwork.png",
  "updated_at": "2026-06-28T15:30:00+09:00"
}
```

## 技術スタック

- **Viewer**: Tauri 2 (macOS) + React + Vite + Tailwind CSS
- **Publisher**: Rust CLI (id3 + mp4ameta)
- **Watch Core**: Rust (notify クレート)

## ADR

設計判断の記録は `docs/adr/` を参照。
