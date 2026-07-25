# ndp-publish

now-dj-playing の楽曲情報書き出しツール (CLI)。楽曲ファイルからタグ・アートワークを抽出し、共有ディレクトリまたは ndp-server に送信する。

## 技術スタック

- Rust
- id3 (MP3 タグ)
- mp4ameta (M4A タグ)
- clap (CLI パーサ)
- ureq (HTTP クライアント、web モード)

## 動作モード

### Local モード (デフォルト)

楽曲ファイルからタグを抽出し、共有ディレクトリに `now_playing.json` + アートワーク + `.ready` を書き出す。viewer がファイル監視でこれを検知する。

### Web モード (`-W`)

ndp-server に HTTP POST で楽曲情報を送信する。セッションの join / publish / leave を行う。アートワークは 800x800 にリサイズして Base64 Data URI で送信。

## インストール

```bash
cd apps/publisher
cargo build --release
```

出力: `target/release/ndp-publish`

## 使い方

```bash
# local モード
ndp-publish --file /path/to/track.mp3 --out ~/ndp --id dj-000 --dj-name "DJ名"

# web モード: join + publish
ndp-publish -W --file /path/to/track.mp3 --dj-name "DJ名" -C 037482

# web モード: join のみ
ndp-publish -W -J --dj-name "DJ名" -C 037482

# web モード: leave
ndp-publish -W -L
```

## オプション

| オプション | 短縮 | 説明 |
|---|---|---|
| `--version` | `-v` | バージョン情報を表示 |
| `--config-file <path>` | `-c` | 設定ファイルのパスを指定 |
| `--web-mode` | `-W` | web モードで動作 |
| `--code <code>` | `-C` | セッション参加用 6 桁コード (web モード) |
| `--join-only` | `-J` | join のみ実行して終了 (web モード) |
| `--leave` | `-L` | セッションから離脱 (web モード) |
| `--file <path>` | `-f` | 楽曲ファイルパス (mp3, m4a) |
| `--out <dir>` | `-o` | 出力先ベースディレクトリ (local モード) |
| `--id <id>` | | DJ ディレクトリ名 (デフォルト: `dj-000`) |
| `--dj-name <name>` | | DJ 名テキスト |

## 設定ファイル (`publish.config.json`)

CLI オプションで指定しない値は設定ファイルから読み込まれる。

### ルックアップ順

1. `-c` オプションで指定されたパス
2. 環境変数 `NDP_PUBLISH_CONFIG`
3. カレントディレクトリの `ndp-publish.config.json`
4. `$HOME/.config/ndp/publish.config.json`

### 形式

```jsonc
{
  "dj_name": "DJ名",
  "local": {
    "dj_id": "dj-000",
    "publish_base_dir": "~/ndp"
  },
  "web": {
    "endpoint_url": "https://relay.example.com"
  }
}
```

## セッションファイル

web モードで join 時にトークンを保存し、以降の publish / leave で使用する。

配置先:
1. 環境変数 `NDP_PUBLISH_SESSION_DIR` で指定されたディレクトリ
2. 設定ファイルと同じディレクトリ

ファイル名: `ndp-publish.session.json`

## ライブラリクレート

`ndp_publish` としてライブラリクレートも公開しており、publisher-gui から利用される。主要モジュール:

- `config` — 設定ファイルの読み込みとルックアップ
- `tags` — 楽曲タグの抽出
- `local` — local モードの publish 処理
- `web` — web モードの join / publish / leave 処理
