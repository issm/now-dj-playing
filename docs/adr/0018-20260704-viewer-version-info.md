# ADR-0018: Viewer にバージョン情報を持たせる

## ステータス

Accepted

## コンテキスト

viewer アプリにバージョン情報がなく、動作中のビルドがどのリビジョンに基づいているか判別できない。デバッグや問い合わせ時にどのバージョンを実行しているか特定する手段が必要。

## 決定

### バージョン書式

SemVer 2.0.0 のビルドメタデータ形式に準拠する:

```
{version}+{yyyymmddThhmmss}.{commit_hash_short}
```

例: `0.1.0+20260704T123045.a1b2c3d`

- `+` 以降はビルドメタデータであり、バージョン比較には影響しない
- タイムスタンプは対象コミットのコミッター日時 (UTC) を使用する（ビルド時刻ではない）
- 開発ビルド時は末尾に `-dev` を付与する（例: `0.1.0+20260704T123045.a1b2c3d-dev`）

### Single Source of Truth

- バージョン番号の正は `apps/viewer/src-tauri/tauri.conf.json` の `version` フィールド
- 以下のファイルはこれに追従させる:
  - `apps/viewer/package.json` の `version`
  - `apps/viewer/src-tauri/Cargo.toml` の `[package] version`

### バックエンド (Rust)

- `build.rs` で `git log` と `git rev-parse` を使いコミット時刻・ハッシュを取得し、`cargo:rustc-env` で環境変数として埋め込む
- `lib.rs` に `get_version_info` Tauri コマンドを追加し、`VersionInfo` 構造体を返す
- デバッグビルド (`cfg!(debug_assertions)`) で `-dev` サフィックスを付与

### フロントエンド (TypeScript)

- `vite.config.ts` で `tauri.conf.json` からバージョンを読み取り、同様に git からコミット情報を取得
- `define` で `__APP_FULL_VERSION__` 等のグローバル定数として注入
- 開発時 (`NODE_ENV !== "production"`) に `-dev` サフィックスを付与

### UI 表示

- メインウィンドウの右下に `Version: {full_version}` を控えめに表示する
- `text-xs text-gray-600` で背景に溶け込むスタイル
- モニタウィンドウには表示しない

## 理由

- SemVer ビルドメタデータ (`+`) を採用した理由: 仕様で定義された標準形式であり、ツールとの互換性が高い。バージョン比較にも影響しない
- コミット時刻を使う理由: 同じコミットから何度ビルドしても同一のバージョン文字列になり、再現性が高い
- `-dev` サフィックスの理由: 開発中のビルドは HEAD から dirty な変更を含む可能性があり、リリースビルドと区別する必要がある
- `tauri.conf.json` を正とした理由: Tauri アプリとしてのバージョンはここで管理されており、ビルドプロセスの起点として最も自然

## 影響

- `build.rs` にコミット情報取得ロジックが追加される
- `lib.rs` に `VersionInfo` 構造体と `get_version_info` コマンドが追加される
- `vite.config.ts` にバージョン情報の注入処理が追加される
- `vite-env.d.ts` にグローバル定数の型宣言が追加される
- `types.ts` に `VersionInfo` インターフェースが追加される
- `App.tsx` に `VersionDisplay` コンポーネントが追加される
- ステアリングファイル `versioning.md` にバージョン管理ルールが記載される
