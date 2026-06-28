# ADR-0001: プロジェクトアーキテクチャ

## ステータス

Accepted

## コンテキスト

「現在再生中の楽曲」に関する情報（曲名、アーティスト名、アルバム、アートワーク）を iPad 上で表示するアプリケーションを構築する。DJ側から共有ディレクトリ経由でファイルとして楽曲情報がプッシュされ、それを監視・解析・表示する。

### 制約

- iPad で利用する
- インターネットにはつながっている（テザリング経由含む）
- LAN にはつながっていない
- 共有ディレクトリ（iCloud Drive 等）経由でデータを受け取る

## 決定

### 技術スタック

- **フレームワーク**: Tauri 2 (iOS ビルド)
- **フロントエンド**: React + Vite + Tailwind CSS
- **iCloud Drive 監視**: Swift による Tauri プラグインとして実装
- **将来の拡張**: WatchProvider インターフェースを抽象化し、Dropbox 等にも対応可能な設計とする

### モノレポ構成

```
now-dj-playing/
├─ apps/
│   ├─ viewer/          ← 表示アプリ (Tauri 2 iOS)
│   └─ publisher/       ← 書き出しツール (後日実装)
├─ packages/
│   └─ shared/          ← 共有スキーマ定義
├─ docs/
│   └─ adr/
└─ ...
```

### 理由

- Tauri 2 の iOS サポートにより、Web 技術 (React) で UI を記述しつつ、Swift ブリッジで iCloud Drive のネイティブ API にアクセスできる
- モノレポ構成により viewer と publisher でスキーマ定義を共有できる
- WatchProvider の抽象化により、将来的なストレージバックエンド追加が容易

## 影響

- iCloud Drive 監視は Tauri プラグイン (Swift) として独自実装が必要
- Tauri 2 の iOS サポートは安定化途上のため、アップデートへの追従が必要
