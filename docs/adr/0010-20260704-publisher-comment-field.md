# ADR-0010: Publisher にコメントフィールドを追加

## ステータス

Accepted

## コンテキスト

楽曲ファイル (mp3/m4a) の ID3/MP4 タグには「コメント」フィールドが含まれていることがある。DJ プレイ時にこの情報を viewer 側で表示したいケースがあり、publisher での抽出が必要になった。

## 決定

### TrackMeta / NowPlaying に `comment` フィールドを追加

- `comment: Option<String>` として定義
- タグにコメントが存在し、かつ空文字列でない場合のみ値を設定する
- `now_playing.json` では `serde(skip_serializing_if = "Option::is_none")` により、値がない場合はフィールド自体を省略する

### コメントの取得方法

| フォーマット | クレート | API |
|---|---|---|
| MP3 (ID3) | `id3` | `tag.comments().next().map(\|c\| c.text.clone())` |
| M4A (MP4) | `mp4ameta` | `tag.comment().map(\|s\| s.to_string())` |

- ID3 の COMM フレームは複数存在しうるが、最初の1件のテキスト部分を採用する
- 空文字列はコメントなしとして扱う（`.filter(|s| !s.is_empty())`）

### now_playing.json の出力例

```json
{
  "title": "曲名",
  "artist": "アーティスト名",
  "album": "アルバム名",
  "artwork": "artwork.png",
  "comment": "コメント内容",
  "updated_at": "2026-07-04T12:00:00+09:00"
}
```

## 理由

- コメントは optional フィールドとすることで、後方互換性を維持する（既存の viewer はフィールドを無視するだけで動作継続可能）
- ID3 の COMM フレームは description/language で区別されるが、DJ 用途では最初の1件で十分と判断
- 空文字列のフィルタにより、viewer 側で不要な空表示を防ぐ

## 影響

- viewer 側で `comment` を表示する場合は別途対応が必要（Issue #4）
- `packages/shared/schemas` の JSON スキーマに `comment` フィールドの定義追加が望ましい
