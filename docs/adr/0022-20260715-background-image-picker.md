# ADR-0022: 背景画像の選択機能（所定ディレクトリの画像一覧から設定）

## ステータス

Accepted（ADR-0020 を Supersede）

## コンテキスト

ADR-0020 では、設定ファイルに単一の `background_image` パスを指定し、`b` キーで表示/非表示をトグルする方式を採用した。しかし運用上、イベントごとに複数の背景候補を用意しておき、配信中にも切り替えたいケースがある。また `show_background_image` による表示/非表示の管理は、「背景画像なし」を選択肢に含めれば不要になる。

## 決定

### 設定ファイルの `background_image` をオブジェクト形式に変更する

```jsonc
{
  "watch_dir": "/tmp/ndp",
  "dj_id": "dj-000",
  "event_name": "Club Night vol.3",
  "enable_comments": true,
  "show_tags": true,
  "background_image": {
    "base_dir": "~/apps/ndp/bg",
    "path": "adamasdining.jpg"   // null で「なし」
  }
}
```

- `background_image.base_dir`: 背景画像を格納するディレクトリ。`~` 展開および設定ファイルからの相対パス解決を行う
- `background_image.path`: `base_dir` からの相対パス。初期選択の画像を指定する。`null` の場合は「なし」（黒背景）で起動

### 廃止するフィールド

- `show_background_image` — 「なし」選択で代替されるため不要

### `background_image` キーが省略された場合

- 背景画像機能は無効とする
- `b` キー押下時、上部アラート（既存の info 帯）に「背景画像ディレクトリが未設定です」を時限表示する
- 一覧オーバーレイは表示しない

### 一覧 UI

- `b` キーで開閉するオーバーレイ
- オーバーレイサイズ: 80vw × 80vh
- `base_dir` 内の画像ファイルを **再帰的に探索** し、サムネイルグリッド（4列）で表示
- サムネイルのアスペクト比はウィンドウのアスペクト比に動的に追従する
- ファイル名はマウスホバー時のみ省略なしで表示する
- 対応形式: png, jpg, jpeg, webp
- 先頭に「なし」の選択肢を配置
- クリックで背景を即時変更
- `Escape` または `b` キーでオーバーレイを閉じる

### 背景の表示方法

- 現行通り: `cover` / `center` / `opacity: 0.15` の独立レイヤー

### 永続化

- セッション限り（アプリ再起動で設定ファイルの `path` に戻る）
- 設定ファイルへの書き戻しは #28 で別途対応する

## 理由

- オブジェクト形式にした理由: ディレクトリと選択画像を明確に分離でき、一覧取得のベースパスが設定に含まれる
- `path` を `base_dir` からの相対パスにした理由: ディレクトリ構造に依存しないポータブルな指定が可能。書き戻し（#28）時にも簡潔な値を保存できる
- `b` キーをトグルから一覧表示に変更した理由: 複数候補から選ぶ操作に統合することで、「表示/非表示」と「画像選択」の2つの操作を1つのキーに集約できる
- `show_background_image` を廃止した理由: 「なし」を選択肢として含めることで、boolean による表示制御が不要になった
- セッション限りとした理由: 設定ファイル書き戻しの仕組みを #28 で統一的に対応するため、本件では切り分ける
- 再帰探索にした理由: サブディレクトリで画像をカテゴリ分けして管理するケースに対応するため
- サムネイルのアスペクト比をウィンドウ比にした理由: 背景として実際にどう表示されるかをプレビューとして直感的に把握できる

## 影響

### Rust 側 (config.rs)

- `AppConfigFile` の `background_image` フィールドを `Option<String>` から `Option<BackgroundImageConfigFile>` に変更
- `show_background_image` フィールドを削除
- `BackgroundImageConfigFile` 構造体を新設（設定ファイル側: `base_dir`, `path`）
- `BackgroundImageConfig` 構造体を新設（解決済み・フロントエンド送信用: `base_dir`, `path`）
- `AppConfig` の背景関連フィールドを更新

### Rust 側 (lib.rs)

- 新コマンド `list_background_images`: `base_dir` 内の画像ファイル一覧を再帰的に探索して返す
- `BackgroundImageEntry` 構造体を新設（`path`: 相対パス, `absolute_path`: 絶対パス）

### フロントエンド (types.ts)

- `AppConfig` から `showBackgroundImage` を削除
- `backgroundImage` を `BackgroundImageConfig | null` 形式に変更
- `BackgroundImageConfig` インターフェースを新設（`baseDir`, `path`）
- `BackgroundImageEntry` インターフェースを新設（`path`, `absolutePath`）

### フロントエンド (App.tsx)

- `b` キーの動作をトグルから一覧オーバーレイ表示に変更
- `showBackgroundImage` state を削除
- `backgroundImageConfig` / `backgroundImagePath` / `backgroundImageAbsolutePath` state を追加

### フロントエンド (新コンポーネント)

- `BackgroundPicker.tsx`: サムネイルグリッドのオーバーレイコンポーネント
  - `useWindowAspectRatio` hook でウィンドウ比に追従

### 設定ファイル

- `development.json.example` を新形式に更新

### ショートカットオーバーレイ

- `b` キーの説明を「背景画像表示のトグル」→「背景画像の選択」に変更
