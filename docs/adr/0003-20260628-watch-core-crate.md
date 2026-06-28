# ADR-0003: watch-core クレートの分離と検証方針

## ステータス

Accepted

## コンテキスト

iCloud Drive のファイル監視は Tauri + Swift プラグインとして実装する計画だが、Swift プラグイン単体での動作確認は困難である（Tauri ランタイムと iOS サンドボックスに依存するため）。開発効率とテスタビリティの観点から、コアロジックを切り出す必要がある。

## 決定

### watch-core を pure Rust クレートとして分離

ファイル監視・解析のコアロジックを `packages/watch-core` に配置し、Tauri に依存しない独立したクレートとする。

```
packages/watch-core/
├─ src/
│   ├─ lib.rs
│   ├─ types.rs       ← 共有型定義 (DjState, WatchEvent 等)
│   ├─ parser.rs      ← .ready / now_playing.json / dj-profile 解析
│   └─ watcher.rs     ← notify クレートによるディレクトリ監視
└─ examples/
    └─ watch_local.rs ← 検証用 CLI
```

### ローカルファイル監視に notify クレートを使用

- macOS / Linux / Windows でクロスプラットフォーム動作
- macOS では FSEvents バックエンドが使われる
- iOS 実機では iCloud Drive 用の Swift プラグインに差し替える想定

### 検証方法

`sandbox/` ディレクトリにダミーデータを配置し、`watch_local` example で動作確認を行う。

```sh
# watcher 起動
cargo run --example watch_local -- ../../sandbox

# 別ターミナルから .ready を更新して検知を確認
touch sandbox/test-dj/.ready
```

### 検証結果

| ケース | 結果 |
|---|---|
| `.ready` の変更検知 | ✓ |
| `now_playing.json` の解析 | ✓ |
| `dj-profile.txt` → DJ 名解決 | ✓ |
| `.ready` の `files` に artwork なし → artwork 無視 | ✓ |
| `.ready` の `files` に artwork あり → パス解決 | ✓ |

### 既知の課題

- ファイル内容書き換え時に notify が複数回イベントを発火する（truncate + write）。必要に応じてデバウンス処理を追加する。

## 理由

- Tauri / Swift に依存しないため、`cargo build` / `cargo test` だけで高速にイテレーションできる
- `notify` クレートによるローカル監視は開発用 `LocalFSWatchProvider` としてそのまま残せる
- 将来 `apps/viewer/src-tauri` から `path` 依存で参照し、Tauri アプリに組み込む

## 影響

- `apps/viewer/src-tauri/Cargo.toml` で `watch-core` を依存に追加する必要がある（結合時）
- iOS 実機では `notify` は使えないため、Swift プラグイン側で同等のイベント発行が必要
