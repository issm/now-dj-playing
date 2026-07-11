# ADR-0020: Viewer に背景画像表示機能を追加する

## ステータス

Accepted

## コンテキスト

DJ イベントの配信画面でブランディングや雰囲気づくりのために、viewer の背景にイベントロゴやフライヤー画像を表示したいケースがある。背景画像はイベント単位で固定される静的な情報であり、設定ファイルで管理するのが適切である。

## 決定

### 設定ファイルに `background_image` と `show_background_image` を追加する

```jsonc
{
  "watch_dir": "/tmp/ndp",
  "dj_id": "dj-000",
  "event_name": "Club Night vol.3",
  "enable_comments": false,
  "show_tags": true,
  "background_image": "/path/to/background.png",
  "show_background_image": true
}
```

- `background_image`: 背景画像ファイルのパス。省略可能で、省略時は背景画像なし（従来どおり黒背景）。`~` はホームディレクトリに展開される
- `show_background_image`: 背景画像を表示するかどうか。デフォルト `true`

### 表示方式

- 背景画像は独立した絶対配置レイヤー（`absolute inset-0 z-0`）として実装する
- `background-size: cover` で要素全体を覆う形で表示
- `opacity: 0.15` を適用し、コンテンツの視認性を確保する
- コンテンツ側は `relative z-10` で背景レイヤーより上に配置する

### キーボードショートカット

- `b` キーで背景画像の表示・非表示をトグルできる

## 理由

- 独立レイヤーにした理由: ルート要素に `opacity` を適用するとアートワークやテキストを含むすべての子要素にも影響してしまうため、背景画像だけを半透明にするには別レイヤーが必要
- `cover` を採用した理由: 全画面背景として隙間なく表示するのが最も自然。画像の一部がトリミングされることは許容する
- `opacity: 0.15` にした理由: 背景を控えめに見せつつ、楽曲情報やアートワークの視認性を損なわないバランス
- `show_background_image` を分離した理由: `background_image` を設定に残したまま一時的に非表示にしたいケースに対応するため
- `b` キーを採用した理由: "background" の頭文字で直感的。既存の `c`（comments）、`t`（tags）、`e`（event）と一貫性がある

## 影響

- `AppConfigFile` / `AppConfig` に `background_image` と `show_background_image` フィールドが追加される
- フロントエンドの `AppConfig` 型に `backgroundImage` と `showBackgroundImage` が追加される
- `App.tsx` に `showBackgroundImage` state と `b` キーショートカットが追加される
- 背景画像レイヤーの描画ロジックが `App` コンポーネントに追加される
- `ShortcutOverlay` に `b` キーの説明が追加される
- 設定ファイル example に `background_image` と `show_background_image` が追加される
