# ADR-0011: Viewer でのコメント構造化表示

## ステータス

Accepted

## コンテキスト

publisher が抽出したコメントフィールドには、DJ が楽曲管理用に記述した構造化テキストが含まれる場合がある。viewer でこの情報を表示する際、生テキストのまま表示するだけでなく、パースして構造化レイアウトで表示したい。

### コメント書式の例

```
魔法の姉妹ルルットリリィ  IN  #anison #2026q2 #maho #jj
ドラゴンボールGT  ED3x  #anison #1996 #1997 #cover #va #vam
#dnb
```

## 決定

### コメントパーサーをフロントエンド (TypeScript) に配置

- `apps/viewer/src/commentParser.ts` としてパーサーを実装
- パース結果は Discriminated Union 型 (`ParsedComment = AnisonComment | GenericComment`) で表現
- 主タグ（1つめのタグ）によって解析結果の型が分岐する設計

### パース結果の型定義

```typescript
interface AnisonComment {
  type: "anison";
  source?: string;       // 作品名
  category?: string;     // OP, ED, IN 等
  yearTags: string[];    // 年代系タグ (数字4桁を含む)
  attrTags: string[];    // 属性系タグ
}

interface GenericComment {
  type: "generic";
  primaryTag: string;    // 主タグそのもの
  source?: string;
  yearTags: string[];
  attrTags: string[];
}

type ParsedComment = AnisonComment | GenericComment;
```

### 表示制御

- 環境変数 `VITE_ENABLE_COMMENTS` (`1` / `0`) で表示の有効・無効を制御
- デフォルトは `0`（非表示）
- パースできないコメントはフォールバックとして生テキスト表示

### レイアウト

- メイン楽曲情報との間にセパレータ（`border-t`）を配置
- 主タグ → 作品名 → カテゴリ → 年代/属性タグ の順に表示
- タグはバッジ風にカラー分類（主タグ: indigo/purple、年代: emerald、属性: gray）

## 理由

### フロントエンドにパーサーを配置した理由

- 現時点でパース結果の消費者は viewer（表示）のみ
- 表示レイアウトとパースロジックが密結合（何をパースするかは表示要件で決まる）
- Vite HMR により表示の試行錯誤が高速
- 将来的に shared パッケージ（Rust / WASM）への移植は容易

### Discriminated Union を採用した理由

- 主タグごとに異なるフィールド構造や表示ロジックを型安全に扱える
- 新しい主タグスキーマの追加が型の追加のみで完結する
- 表示コンポーネントの分岐が `parsed.type` で明確に行える

## 影響

- 新しい主タグの解析構造を追加する場合は、型定義 + パーサー分岐 + 表示コンポーネントの3点を追加する
- パーサーのロジック変更はフロントエンドのみで完結（バックエンドのリビルド不要）
