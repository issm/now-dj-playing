# ADR-0035: macOS アプリの配布方針

## ステータス

Accepted

## コンテキスト

Tauri アプリ（publisher-gui、viewer）を macOS ユーザに配布する必要がある。macOS では Gatekeeper がダウンロードされたアプリの署名と公証を検証し、未署名アプリの起動をブロックする。

配布方法として以下の選択肢がある:

| 方法 | 署名 | 公証 | ユーザ体験 |
|------|------|------|-----------|
| A. Ad-hoc 署名 | `codesign --sign -` | なし | ユーザが `xattr -cr` を実行する必要あり |
| B. Developer ID 署名 | Developer ID Application 証明書 | なし | システム設定から「このまま開く」で起動可能 |
| C. Developer ID 署名 + 公証 | Developer ID Application 証明書 | Apple notarytool | 追加操作なしで起動可能 |

## 決定

### 署名方式: Ad-hoc 署名（方法 A）を採用

- 小規模な配布（限られたユーザへの手渡し）であるため、Developer ID 署名・公証のコストは不要
- ビルド後に `codesign --force --deep --sign -` を実行
- ユーザには初回起動前に `xattr -cr` の実行を案内する

### ビルドターゲット: aarch64-apple-darwin (Apple Silicon)

- ビルドマシン: M4 Mac（Apple Silicon）
- 配布先: M1 Mac ユーザ
- 同一アーキテクチャ (aarch64) のため、ネイティブビルドそのままで動作する
- Intel Mac 対応（ユニバーサルバイナリ）は現時点では不要

### 配布形式: .app (zip 圧縮)

- `.app` バンドルを zip 圧縮して配布
- DMG も署名なしでは結局 quarantine 対象のため、zip で十分

### リリースフロー (`bin/release-local.sh`)

全アプリを一括でビルド・署名・配置するスクリプトを用意:

```bash
bin/release-local.sh <RELEASE_BASEDIR>
```

処理内容:

| 対象 | ビルド | 成果物 | 署名 |
|------|--------|--------|------|
| server | `cargo build --release` | `ndp-server` バイナリ | 不要 |
| viewer | `yarn tauri build` | `now-dj-playing.app` | `codesign --force --deep --sign -` |
| publisher | `cargo build --release` | `ndp-publish` バイナリ | 不要 |
| publisher-gui | `yarn tauri build` | `ndp-publish-gui.app` | `codesign --force --deep --sign -` |

成果物は `<RELEASE_BASEDIR>/<app名>/` に配置される。

### ユーザ向けセットアップスクリプト

アプリごとにセットアップスクリプトを用意し、配布物に同梱する:

- `bin/setup-publisher-gui.mac.sh` — publisher-gui 用
- `bin/setup-viewer.mac.sh` — viewer 用

いずれも quarantine 属性を除去するのみ:

```bash
#!/bin/bash
APP_NAME="<app名>.app"
xattr -cr "./$APP_NAME"
echo "セットアップ完了"
```

### ユーザ向け案内

1. 配布された zip を展開
2. `.app` と `setup.mac.sh` を同一ディレクトリに配置
3. ターミナルで `bash setup-<app名>.mac.sh` を実行
4. `.app` をダブルクリックで起動

アップデート時も同様に zip 展開 → セットアップスクリプト実行が必要。

## 理由

- **Ad-hoc 署名を選択した理由**: 配布先が少数の既知ユーザに限られるため、公証の手間（App 用パスワード管理、ビルド時間増加）に見合わない
- **zip を選択した理由**: DMG にしても署名なしでは quarantine の問題は同じ。zip のほうがビルドが簡単で配布サイズも小さい
- **セットアップスクリプト同梱**: `xattr -cr` コマンドをユーザに覚えてもらうより、スクリプトとして渡すほうが確実
- **リリーススクリプトで一括管理**: ビルド後の署名忘れを防止し、全成果物を統一的に管理する

## 将来の検討事項

- 配布先が増えた場合は Developer ID 署名 + 公証（方法 C）へ移行を検討
- GitHub Releases での公開時は公証が強く推奨される
- Tauri v2 は `APPLE_SIGNING_IDENTITY` / `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` 環境変数で署名・公証を自動実行可能

## 影響

- `bin/release-local.sh` — 全アプリのビルド + 署名 + 配置を一括実行
- `bin/setup-publisher-gui.mac.sh` — publisher-gui 用セットアップスクリプト
- `bin/setup-viewer.mac.sh` — viewer 用セットアップスクリプト
- アップデート配布時はユーザ側で再度セットアップスクリプトの実行が必要（案内に明記）
