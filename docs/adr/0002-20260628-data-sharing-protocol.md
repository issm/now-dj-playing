# ADR-0002: データ共有プロトコル

## ステータス

Accepted

## コンテキスト

DJ側アプリケーションから iPad 表示アプリへ、iCloud Drive 等の共有ディレクトリを経由して楽曲情報を伝達する必要がある。iCloud Drive のファイル同期はファイル単位で非同期に行われるため、複数ファイルの整合性を保証する仕組みが必要。

## 決定

### ディレクトリ構成

```
{base_dir}/
├─ {dj-directory}/
│   ├─ dj-profile[.txt|.png|.jpg|.jpeg]
│   ├─ now_playing.json
│   ├─ artwork.{png|jpg|jpeg}  (optional)
│   └─ .ready
└─ {dj-directory}/
    └─ ...
```

ベースディレクトリ配下を再帰的に監視する。

### now_playing.json スキーマ

```json
{
  "title": "曲名",
  "artist": "アーティスト名",
  "album": "アルバム名",
  "artwork": "artwork.png",
  "updated_at": "2026-06-28T15:30:00+09:00"
}
```

- `artwork` フィールドは同ディレクトリ内の画像ファイル名を参照
- アートワークなしの場合は `artwork` を null または省略

### .ready ファイル (sentinel / マニフェスト)

```json
{
  "updated_at": "2026-06-28T15:30:00+09:00",
  "files": ["now_playing.json", "artwork.png"]
}
```

- 書き出し側は全ファイル書き出し後、最後に `.ready` を作成/更新する
- 表示側は `.ready` の出現/更新を検知したタイミングでのみ読み取りを実行する
- `files` に含まれるファイルだけを読む（過去のアートワーク誤表示を防止）

### DJ プロファイル

| ファイル | 解釈 |
|---|---|
| `dj-profile` (拡張子なし) | テキスト → 内容を DJ 名として使用 |
| `dj-profile.txt` | テキスト → 同上 |
| `dj-profile.png` / `.jpg` / `.jpeg` | 画像 → DJ ロゴとして表示 |

**DJ 名の解決優先順位:**

1. `dj-profile` / `dj-profile.txt` のテキスト内容
2. 画像ロゴが存在する場合、DJ 名はディレクトリ名をフォールバック
3. プロファイルが存在しない場合、ディレクトリ名をそのまま DJ 名として表示

### 書き出しフロー

1. `now_playing.json` を書き出す
2. artwork があれば画像ファイルを書き出す
3. `.ready` を書き出す（`files` に 1, 2 で書いたファイル名を列挙）

### 表示側フロー

1. `.ready` の変更を検知
2. `.ready` を読み `files` リストを取得
3. `files` に基づき `now_playing.json` を読む
4. `files` に artwork が含まれれば画像を読む（含まれなければアートワークなし）
5. `dj-profile` を読む（初回 or プロファイル変更時）
6. 表示更新 & 履歴に追加

## 理由

- sentinel ファイル方式により、非同期ファイル同期環境での整合性を保証
- マニフェスト (`files`) により、前回の残存ファイルを誤って参照することを防止
- dj-profile の柔軟なフォーマット対応で、テキスト名 / ロゴ画像のどちらでも DJ を表現可能

## 影響

- 書き出し側は `.ready` を最後に書くという規約を守る必要がある
- dj-profile は楽曲更新とは別ライフサイクル（頻繁には変わらない）
