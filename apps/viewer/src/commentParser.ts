/** anison 主タグの解析結果 */
export interface AnisonComment {
  type: "anison";
  source?: string;
  category?: string;
  yearTags: string[];
  attrTags: string[];
}

/** 未定義の主タグ用フォールバック */
export interface GenericComment {
  type: "generic";
  primaryTag: string;
  source?: string;
  yearTags: string[];
  attrTags: string[];
}

export type ParsedComment = AnisonComment | GenericComment;

/** カテゴリとして認識するパターン (OP, ED, IN, IM + オプショナルな x/数字/#数字) */
const CATEGORY_PATTERN =
  /^(OP|ED|IN|IM)(x|\d+x?)?(#\d+(-\d+)?(,\d+)*)?$/i;

/** カンマ区切りの複合カテゴリ判定 (例: "ED#18-19,BD#12-13") */
const COMPOUND_CATEGORY_PATTERN =
  /^((OP|ED|IN|IM)(x|\d+x?)?(#\d+(-\d+)?(,\d+)*)?)(,((OP|ED|IN|IM)(x|\d+x?)?(#\d+(-\d+)?(,\d+)*)?))*$/i;

/** 年代系タグの判定: 数字4桁を含む */
const YEAR_TAG_PATTERN = /\d{4}/;

/**
 * コメント文字列を構造化する。
 * タグが1つも含まれない場合は null を返す。
 */
export function parseComment(raw: string): ParsedComment | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  // `/` 区切りの複数エントリがある場合、最初のエントリのみ採用
  const entry = trimmed.split(/\s+\/\s+/)[0]?.trim();
  if (!entry) return null;

  // タグの開始位置を特定: 空白の直後に # が来る最初の位置
  // カテゴリ内の # (例: IN#5) と区別するため
  const tagStartMatch = entry.match(/(?:^|\s)(#\S)/);
  if (!tagStartMatch) return null;

  const tagStartIndex = entry.indexOf(tagStartMatch[0]) + tagStartMatch[0].indexOf("#");
  const prefix = entry.slice(0, tagStartIndex).trim();
  const tagsPart = entry.slice(tagStartIndex);

  // タグを抽出 (#で始まる連続非空白文字)
  const tagMatches = tagsPart.match(/#[^\s#]+/g);
  if (!tagMatches || tagMatches.length === 0) return null;

  // # を除去したタグ配列
  const tags = tagMatches.map((t) => t.slice(1));

  const primaryTag = tags[0];
  if (!primaryTag) return null;
  const restTags = tags.slice(1);

  // 年代系と属性系に分類
  const yearTags: string[] = [];
  const attrTags: string[] = [];
  for (const tag of restTags) {
    if (YEAR_TAG_PATTERN.test(tag)) {
      yearTags.push(tag);
    } else {
      attrTags.push(tag);
    }
  }

  // プレフィックスからカテゴリを分離
  const { source, category } = parsePrefix(prefix);

  // 主タグに応じて型を分岐
  if (primaryTag === "anison") {
    return { type: "anison", source, category, yearTags, attrTags };
  }

  return { type: "generic", primaryTag, source, yearTags, attrTags };
}

/**
 * プレフィックス部分を source と category に分離する。
 * 末尾トークンがカテゴリパターンにマッチすればカテゴリとして扱う。
 */
function parsePrefix(prefix: string): {
  source?: string;
  category?: string;
} {
  if (!prefix) return {};

  // 末尾から2トークンまでをカテゴリ候補として試す
  // 例: "ARIA The NATURAL  ED2" → source="ARIA The NATURAL", category="ED2"
  // 例: "ニセコイ  ED#18-19,BD#12-13" → source="ニセコイ", category="ED#18-19,BD#12-13"
  const tokens = prefix.split(/\s+/);
  if (tokens.length === 0) return {};

  const lastToken = tokens[tokens.length - 1];
  if (!lastToken) return {};

  if (CATEGORY_PATTERN.test(lastToken)) {
    const source = tokens.slice(0, -1).join(" ") || undefined;
    return { source, category: lastToken };
  }

  // カンマ区切りの複合カテゴリ (例: "ED#18-19,BD#12-13")
  if (lastToken.includes(",") && COMPOUND_CATEGORY_PATTERN.test(lastToken)) {
    const source = tokens.slice(0, -1).join(" ") || undefined;
    return { source, category: lastToken };
  }

  // カテゴリなし → 全体が source
  return { source: prefix };
}
