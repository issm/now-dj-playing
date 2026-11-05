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

### ビルドターゲット

#### aarch64-apple-darwin (Apple Silicon)

- ビルドマシン: M4 Mac（Apple Silicon）
- 配布先: M1 Mac ユーザ
- 同一アーキテクチャ (aarch64) のため、ネイティブビルドそのままで動作する

#### x86_64-apple-darwin (Intel Mac) — publisher-gui のみ (#90)

Intel Mac の配布先が出たため、publisher-gui に限り Intel 向けビルドを追加した。

- 配布先: Intel Mac (MacBook Pro 13 インチ 2017 / Intel Core i5)
- ビルド: Apple Silicon 上で `yarn tauri build --target x86_64-apple-darwin` によるクロスビルド
  - 事前に `rustup target add x86_64-apple-darwin` が必要
  - 成果物は `src-tauri/target/x86_64-apple-darwin/release/bundle/macos/` 配下に出力される
- ユニバーサルバイナリ (`universal-apple-darwin`) は採用しない
  - 配布先のアーキテクチャが分かっており、サイズ（約 2 倍）とビルド時間の増加に見合わない
  - 配布物は arm64 版と x86_64 版を別ディレクトリで管理する
- **クロスビルドの成果物は署名されない**ため、ad-hoc 署名の実行が必須（`scripts/release-local.sh` に組み込み済み）
- viewer の Intel 対応は未実施

### 配布形式: .app (zip 圧縮)

- `.app` バンドルを zip 圧縮して配布
- DMG も署名なしでは結局 quarantine 対象のため、zip で十分

### リリースフロー (`scripts/release-local.sh`)

全アプリを一括でビルド・署名・配置するスクリプトを用意:

```bash
scripts/release-local.sh <RELEASE_BASEDIR>
```

処理内容:

| 対象 | ビルド | 成果物 | 署名 |
|------|--------|--------|------|
| server | `cargo build --release` | `ndp-server` バイナリ | 不要 |
| viewer | `yarn tauri build` | `now-dj-playing.app` | `codesign --force --deep --sign -` |
| publisher | `cargo build --release` | `ndp-publish` バイナリ | 不要 |
| publisher-gui | `yarn tauri build` | `ndp-publish-gui.app` | `codesign --force --deep --sign -` |
| publisher-gui (darwin) | `yarn tauri build --target x86_64-apple-darwin` | `ndp-publish-gui.app` (Intel) | `codesign --force --deep --sign -` |

成果物は `<RELEASE_BASEDIR>/<app名>/` に配置される。publisher-gui (darwin) のみ `<RELEASE_BASEDIR>/publisher-gui-darwin/` に配置される。

### ユーザ向けセットアップスクリプト

アプリごとにセットアップスクリプトを用意し、配布物に同梱する:

- `scripts/setup-publisher-gui.mac.sh` — publisher-gui 用（Intel 版も共通）
- `scripts/setup-viewer.mac.sh` — viewer 用

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
- **Intel 版を別ビルドにした理由**: 配布先が少数で既知のため、ユニバーサルバイナリにするメリットがない。個別ビルドのほうが成果物が小さい

## 将来の検討事項

- 配布先が増えた場合は Developer ID 署名 + 公証（方法 C）へ移行を検討
- GitHub Releases での公開時は公証が強く推奨される
- Tauri v2 は `APPLE_SIGNING_IDENTITY` / `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` 環境変数で署名・公証を自動実行可能
- 配布先のアーキテクチャが混在して管理が煩雑になった場合は、ユニバーサルバイナリへの一本化を検討
- viewer の Intel 対応が必要になった場合は、`release-local.sh` に同様のセクションを追加

## 影響

- `scripts/release-local.sh` — 全アプリのビルド + 署名 + 配置を一括実行（publisher-gui (darwin) セクションを追加）
- `scripts/setup-publisher-gui.mac.sh` — publisher-gui 用セットアップスクリプト
- `scripts/setup-viewer.mac.sh` — viewer 用セットアップスクリプト
- アップデート配布時はユーザ側で再度セットアップスクリプトの実行が必要（案内に明記）
