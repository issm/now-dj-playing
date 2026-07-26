# ADR-0034: DJ 画像の全レイヤー対応と設定構造 base 空間の導入

## ステータス

Accepted

## コンテキスト

#68 で publisher-gui に DJ 画像の設定・プレビュー機能を追加したが、web モードの join/publish フローでは画像が送信されず、viewer のロスター表示にも反映されなかった。また設定ファイルの構造として `dj_name` がトップレベルに置かれており、`dj_image` との一貫性がなかった。

## 決定

### 1. 設定構造に `base` 空間を導入

```json
{
  "base": {
    "dj_name": "いわさん",
    "dj_image": "/path/to/image.png"
  },
  "local": { ... },
  "web": { ... }
}
```

- `base` 空間に DJ 名・DJ 画像などの共通設定を格納する
- 後方互換: トップレベルの `dj_name` / `dj_image` も読み込み対象（`base` 内を優先してフォールバック）
- publisher-gui の Config 保存は新構造 (`base` 空間) で書き出す

### 2. web モード: join 時に dj_image を送信

- publisher: `JoinRequest` に `dj_image` フィールドを追加
- 画像は 800x800 px に収まるようリサイズし、Base64 Data URI として送信
- server: `Publisher` 構造体に `dj_image: Option<String>` を追加
- SSE `publisher_joined` イベントに `dj_image` を含めて配信

### 3. local モード: publish 時に dj_image を出力

- `publish_local` に `dj_image: Option<&Path>` パラメータを追加
- 画像優先: `dj_image` が指定されていれば `dj_name` よりも優先して DJ プロファイルに使用
- 画像ファイルを `dj-profile.{ext}` としてコピー

### 4. viewer: ロスター表示で dj_image を利用

- `DjJoined` インターフェースに `djImage: string | null` を追加
- ロスター (`roster`) のデータ構造を `Map<string, { djName, djImage }>` に拡張
- `DjRosterHeader` で各 DJ の画像を表示（Data URI に対応）
- 複数 DJ 時も個別に画像を表示

### 5. 優先度ルール

DJ 名が画像・テキスト両方設定されている場合: **画像 > テキスト**

## データフロー

```
publisher                 server                    viewer
─────────────────────────────────────────────────────────────
join(code, dj_name,    → Publisher{dj_image}     → SSE publisher_joined
     dj_image)           保存                       {dj_image} → ロスター表示
```

## 画像処理

- リサイズ: 800x800 px に収まるよう Lanczos3 フィルタで縮小
- フォーマット: 元の形式 (PNG/JPEG) を維持
- エンコード: Base64 Data URI (`data:{mime};base64,{data}`)
- web モード: join 時のみ送信（publish 時には含めない）
- local モード: publish のたびにファイルコピー

## viewer 表示ルール

- DJ 画像が指定されている場合: **画像のみ表示**（テキスト名は非表示）
- DJ 画像が未指定の場合: テキスト名を表示
- 画像は全体を表示（`object-contain`）、ロスター領域の高さに収める
- 複数 DJ 時も各 DJ 個別に画像 or テキストを出し分ける

## join_only のオーバーライド

`web::join_only` は `dj_image_override: Option<&Path>` パラメータを持つ。

- CLI: `None` を渡し、`AppConfig` の `dj_image_path()` を使用
- publisher-gui: フロントエンドの最新の `djImage` ステートをパスとして渡す
  - GUI 上で画像を差し替えた後の再 join で、古い config のキャッシュではなく最新のパスが使用される

## 影響

- `apps/publisher/src/config.rs`: `BaseConfig` 構造体追加、`AppConfig` に `dj_image()` / `dj_image_path()` メソッド追加
- `apps/publisher/src/web.rs`: `JoinRequest` に `dj_image` 追加、`load_dj_image_data_uri` 関数追加
- `apps/publisher/src/local.rs`: `publish_local` シグネチャ変更（`dj_image` パラメータ追加）
- `apps/server/src/session.rs`: `Publisher` に `dj_image` 追加、`join_by_code` シグネチャ変更
- `apps/viewer/src/useDataSource.ts`: `DjJoined` に `djImage` 追加
- `apps/viewer/src/App.tsx`: ロスター構造変更、`DjRosterHeader` で画像表示対応
- `apps/publisher-gui/src-tauri/src/lib.rs`: `base` 空間形式で Config 保存

## 関連

- [ADR-0029](./0029-20260723-publisher-gui.md) — Publisher GUI アプリ（#68 での DJ 画像 GUI 対応）
- [#68](https://github.com/issm/now-dj-playing/issues/68) — publisher-gui: 基本タブ追加と DJ 名画像対応
- [#70](https://github.com/issm/now-dj-playing/issues/70) — web モードで DJ 名画像を join/publish フローに対応させる
