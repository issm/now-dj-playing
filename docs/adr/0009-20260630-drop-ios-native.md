# ADR-0009: iOS ネイティブ対応の取りやめ

## ステータス

Accepted

## コンテキスト

当初の設計（ADR-0001）では Tauri 2 の iOS ビルドで iPad 上にネイティブアプリとして viewer を配布する方針だった。しかし運用検討を進める中で以下が明らかになった:

- Phase 1（現行）では Mac の Sidecar（画面ミラーリング）で iPad をセカンダリモニタとして利用しており、iPad にアプリをインストールする必要がない
- Phase 2 以降で中継サーバを導入する場合、viewer は WebSocket で JSON を受け取って表示するだけであり、ブラウザ（Safari）上の Web アプリで十分に実現できる
- iOS ネイティブビルドには TestFlight 配布・Apple Developer Program 費用・Xcode でのアーカイブ作業等のオーバーヘッドが伴う
- viewer のフロントエンドは React + Vite で構築されており、Web アプリとしてそのまま配信可能な素地がある

## 決定

iOS ネイティブアプリとしての viewer 開発は行わない。

- Phase 1: Sidecar で Mac の viewer ウィンドウを iPad に表示
- Phase 2 以降: ブラウザ（Safari）で動作する Web 版 viewer を提供

viewer の Tauri シェルは macOS 向けのみとする。

## 理由

- iOS ネイティブビルドのオーバーヘッド（署名・配布・レビュー）を排除できる
- ブラウザ版であれば iPad に限らずモニタ付きの任意のデバイスで表示可能
- 既存の React フロントエンドを Web アプリとして流用できる可能性がある（詳細は Phase 2 着手時に検討）

## 影響

- ADR-0001 の「Tauri 2 iOS ビルド」「iCloud Drive Swift プラグイン」の方針を上書きする
- `src-tauri` の iOS 関連設定（`gen/apple` 等）は将来的に削除を検討
- Xcode は macOS ビルドのために引き続き必要
