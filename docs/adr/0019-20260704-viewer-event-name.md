# ADR-0019: Viewer にイベント名を表示できるようにする

## ステータス

Accepted

## コンテキスト

DJ イベントで viewer を利用する際、現在どのイベントで配信しているかを画面上に表示したいケースがある。イベント名は楽曲が変わるたびに変化するものではなく、イベント単位で固定される静的な情報であるため、設定ファイルで管理するのが適切である。

## 決定

### 設定ファイルに `event_name` と `show_event_name` を追加する

```jsonc
{
  "watch_dir": "/tmp/ndp",
  "dj_id": "dj-000",
  "event_name": "Club Night vol.3",
  "show_event_name": true,
  "enable_comments": false,
  "show_tags": true
}
```

- `event_name`: イベント名の文字列。省略可能で、省略時は領域自体が非表示になる
- `show_event_name`: イベント名を表示するかどうか。デフォルト `true`

### 表示位置

- DJ 名ヘッダの上部にイベント名領域を配置する
- `event_name` が未設定または空文字の場合は領域自体がレンダリングされない
- `show_event_name` が `false` の場合も非表示

### キーボードショートカット

- `e` キーでイベント名の表示・非表示をトグルできる

### DJ 名ヘッダの高さ

- 固定値 `100px` に変更（以前は `15vh`）

## 理由

- 設定ファイルで管理する理由: イベント名は publisher の push ごとに変化しない静的情報であり、バックエンドのイベント駆動で変更する必要がない
- `show_event_name` を分離した理由: `event_name` を設定に残したまま一時的に非表示にしたいケースに対応するため
- `e` キーを採用した理由: "event" の頭文字で直感的。既存の `c`（comments）、`t`（tags）と一貫性がある
- ヘッダ高さを固定値にした理由: `vh` 単位だとウィンドウサイズによって過大/過小になるため、安定した見た目を確保する

## 影響

- `AppConfigFile` / `AppConfig` に `event_name` と `show_event_name` フィールドが追加される
- フロントエンドの `AppConfig` 型に `eventName` と `showEventName` が追加される
- `App.tsx` に `showEventName` state と `e` キーショートカットが追加される
- `TrackDisplay` コンポーネントにイベント名表示領域が追加される
- `ShortcutOverlay` に `e` キーの説明が追加される
- 設定ファイル example に `event_name` と `show_event_name` が追加される
