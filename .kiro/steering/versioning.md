---
inclusion: auto
---

# バージョン管理規約

## バージョン書式

SemVer 2.0.0 に準拠し、ビルドメタデータを付与する:

```
{version}+{yyyymmddThhmmss}.{commit_hash_short}
```

例: `0.1.0+20260704T123045.a1b2c3d`

開発ビルド時は末尾に `-dev` を付与する:

```
{version}+{yyyymmddThhmmss}.{commit_hash_short}-dev
```

例: `0.1.0+20260704T123045.a1b2c3d-dev`

## Single Source of Truth

- バージョン番号の正は `apps/viewer/src-tauri/tauri.conf.json` の `version` フィールド
- 以下のファイルは `tauri.conf.json` に追従させること:
  - `apps/viewer/package.json` の `version`
  - `apps/viewer/src-tauri/Cargo.toml` の `[package] version`

## ビルドメタデータ

- ビルド時に自動生成される（手動管理不要）
- タイムスタンプは**対象コミット (HEAD) のコミッター日時 (UTC)** を使用する（ビルド実行時刻ではない）
  - 同じコミットから何度ビルドしても同じバージョン文字列になる（再現性）
- 構成: `{コミット日時UTC}.{git commit hash short}`
- Rust 側: `build.rs` が `cargo:rustc-env` で `BUILD_METADATA`, `BUILD_TIMESTAMP`, `BUILD_COMMIT_HASH`, `BUILD_DEV_SUFFIX` を設定
- フロントエンド側: `vite.config.ts` が `__APP_FULL_VERSION__` 等のグローバル定数として注入

## 開発ビルドの判定

- Rust 側: `cfg!(debug_assertions)` で判定
- フロントエンド側: `process.env.NODE_ENV` が `"production"` でない場合に `-dev` を付与

## バージョンを上げるとき

1. `apps/viewer/src-tauri/tauri.conf.json` の `version` を更新する
2. `apps/viewer/package.json` の `version` を同じ値に更新する
3. `apps/viewer/src-tauri/Cargo.toml` の `[package] version` を同じ値に更新する
4. コミットメッセージ例: `chore(viewer): バージョンを 0.2.0 に更新`
