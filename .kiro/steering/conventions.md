# プロジェクト規約

## Git

- コミットメッセージは Conventional Commits 形式で、日本語で書く
  - 例: `feat(viewer): 既存データスキャン対応`
  - 例: `docs: ADR-0003 watch-core クレートの分離と検証方針`
  - 例: `chore: ステアリング追加 (プロジェクト規約)`
- スコープは `viewer`, `publisher`, `watch-core` 等のパッケージ名を使用する
- コミットは指示があるまで行わない
- issue 対応時は、`issues/` プレフィックス + issue ID + 対応内容に関するワードで構成されたブランチを作成して進行する
  - 例: `issues/3-publisher-comment-field`, `issues/4-viewer-show-comment`
- issue 対応におけるコミットメッセージには `#{issue_id}` を含める
  - 例: `feat(publisher): コメントフィールドの抽出を追加 #3`

## パッケージマネージャ

- Node.js のパッケージマネージャは yarn (v1) を使用する
- npm は使用しない

## 開発環境

- direnv を使用している。ターミナルでのコマンド実行時は `.envrc` の内容を考慮すること
- Node.js のバージョン管理は nodenv を使用。`NODENV_VERSION` 環境変数で指定する
- JSON の整形・抽出には `jq` を使用する（`python3 -m json.tool` ではなく）

## ファイル編集

- 既存ファイルへの変更が複数箇所にわたる場合は、`str_replace` の複数回呼び出しではなく `fs_write` でファイル全体を書き込むこと
  - supervised mode での hunk 単位 accept/reject が不安定なため、1回の書き込みで完結させる

## 言語

- ドキュメント・コメントは日本語を基本とする
- コード中の識別子は英語
