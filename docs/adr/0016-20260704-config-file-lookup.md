# ADR-0016: 設定ファイルのルックアップ方式による構成管理

## ステータス

Accepted

## コンテキスト

viewer アプリはこれまで Vite の環境変数（`VITE_WATCH_DIR`, `VITE_DEFAULT_DJ_ID`, `VITE_ENABLE_COMMENTS`）を用いて起動時設定を管理していた。しかし Vite の環境変数はビルド時にフロントエンドへ埋め込まれる仕組みのため、リリースビルド（`yarn tauri build`）で生成した `.app` では `.env.development` が読み込まれず、設定値が未定義になる問題が発生していた（#16）。

また、エラーメッセージに「`.env.development を確認してください`」と表示されるが、リリースビルドの文脈では不適切だった。

## 決定

### 設定をバックエンド（Rust 側）で管理する

フロントエンドの `VITE_*` 環境変数による設定管理を廃止し、Rust 側で設定ファイルを読み込み、Tauri コマンド経由でフロントエンドに提供する。

### 設定ファイル形式

- JSONC（JSON with Comments）形式。`//` および `/* */` コメント、trailing comma を許容する
- 拡張子は `.json`
- パーサーとして `serde_jsonc` クレートを使用する

```jsonc
{
  // 監視対象ディレクトリ（~ はホームディレクトリに展開される）
  "watch_dir": "~/ndp",
  "dj_id": "dj-000",
  "enable_comments": false
}
```

### キー命名規則

- snake_case を採用する
- 理由: Rust の serde デフォルトとの親和性、ユーザーが手編集する際の可読性

### ルックアップ順

以下の順序で設定ファイルを探索し、最初に見つかったものを使用する:

1. 環境変数 `NDP_CONFIG` で指定されたパスのファイル
2. 実行バイナリに隣接する `ndp.config.json`
3. `$HOME/.config/ndp/config.json`
4. いずれも見つからない場合はデフォルト値で動作する

### デフォルト値

| キー | デフォルト値 |
|---|---|
| `watch_dir` | `~/ndp` |
| `dj_id` | `"dj-000"` |
| `enable_comments` | `false` |

### `~` 展開

`watch_dir` の値に `~` が含まれる場合、`shellexpand` クレートを用いてホームディレクトリに展開する。

### Tauri コマンド

- `get_app_config` コマンドを新設し、フロントエンドに設定を提供する
- フロントエンド向けのレスポンスは camelCase でシリアライズする
- `start_watch` コマンドはフロントエンドから `baseDir` を受け取る形式を廃止し、バックエンド内部で設定から取得する

### フロントエンドの変更

- 起動時に `invoke("get_app_config")` で設定を取得する
- `VITE_WATCH_DIR`, `VITE_DEFAULT_DJ_ID`, `VITE_ENABLE_COMMENTS` はすべて廃止する
- エラーメッセージは設定ファイルの文脈に合わせて修正する

### 開発時の運用

- `.envrc` に `export NDP_CONFIG="$PWD/apps/viewer/src-tauri/config/development.json"` を追加する
- `apps/viewer/src-tauri/config/development.json.example` をリポジトリに含める
- 開発者は example をコピーして `development.json` を作成し、自身の環境に合わせて編集する

### 追加する Rust 依存

- `serde_jsonc` — JSONC パース
- `shellexpand` — `~` のホームディレクトリ展開

## 理由

- バックエンドで設定を管理する理由: Vite の環境変数はビルド時埋め込みであり、リリースビルドとの相性が根本的に悪い。Rust 側で実行時に読み込めばビルド形態に依存しない
- ルックアップ方式を採用する理由: 開発時（環境変数）、ポータブル配布（バイナリ隣接）、通常インストール（`$HOME/.config`）それぞれのユースケースに対応できる
- JSONC を採用する理由: 設定ファイルはユーザーが手編集するものであり、コメントで各項目の説明を記述できると利便性が高い。VSCode と同じセマンティクスで馴染みがある
- `.env.production` 方式を不採用とした理由: `~` 展開ができず、全ユーザー同一パスになる前提が非現実的
- 設定 UI を今回不採用とした理由: スコープが大きい。将来的な拡張として検討する
- `NDP_CONFIG` 環境変数を最優先とした理由: CI やテスト環境での柔軟な切り替え、および `direnv` との組み合わせで開発体験が良い

## 影響

- `apps/viewer/.env.development`, `.env.example` は設定としての役割を失う（フロントエンド固有の設定が残る場合のみ維持）
- `.envrc` / `.envrc.example` に `NDP_CONFIG` の記述が追加される
- `apps/viewer/src-tauri/config/` ディレクトリが新設される
- 将来的に設定項目を追加する場合は、設定ファイルのスキーマとデフォルト値の両方を更新する
