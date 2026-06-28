# プロジェクト規約

## Git

- コミットメッセージは Conventional Commits 形式で、日本語で書く
  - 例: `feat(viewer): 既存データスキャン対応`
  - 例: `docs: ADR-0003 watch-core クレートの分離と検証方針`
  - 例: `chore: ステアリング追加 (プロジェクト規約)`
- スコープは `viewer`, `publisher`, `watch-core` 等のパッケージ名を使用する
- コミットは指示があるまで行わない

## パッケージマネージャ

- Node.js のパッケージマネージャは yarn (v1) を使用する
- npm は使用しない

## 開発環境

- direnv を使用している。ターミナルでのコマンド実行時は `.envrc` の内容を考慮すること
- Node.js のバージョン管理は nodenv を使用。`NODENV_VERSION` 環境変数で指定する

## 言語

- ドキュメント・コメントは日本語を基本とする
- コード中の識別子は英語
