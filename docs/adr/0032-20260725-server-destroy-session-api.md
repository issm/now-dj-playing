# ADR-0032: セッション破棄 API (`DELETE /sessions/{id}`) の実装

## ステータス

Accepted

## コンテキスト

現状、セッションの削除手段がなく、viewer が接続を切った後もセッションがサーバーのメモリ上に残り続ける（ゾンビセッション問題）。

- viewer が設定再読み込みや終了を行うたびに新しいセッションが作成されていた
- 古いセッションは一切クリーンアップされない
- サーバーを再起動するまでメモリリークが蓄積する

## 決定

### エンドポイント

```
DELETE /api/sessions/{session_id}
```

### 認証

viewer の Bearer トークン (`Authorization: Bearer vt_xxxxxxxxxxxx`) で認証する。セッション作成時に発行された `viewer_token` のみが破棄を許可される。

### 処理フロー

1. Authorization ヘッダから Bearer トークンを抽出
2. パスの `session_id` に該当するセッションを取得
3. セッションの `viewer_token` とリクエストのトークンが一致することを検証
4. セッションおよび対応する broadcast チャネルを削除
5. `204 No Content` を返す

broadcast チャネルの削除により、SSE 接続中の receiver は自然に切断される。

### エラーレスポンス

| ステータス | 条件 |
|---|---|
| 401 Unauthorized | Authorization ヘッダが未指定 |
| 403 Forbidden | viewer_token が不一致 |
| 404 Not Found | セッションが見つからない |

### 実装構成

- `src/session.rs`:
  - `SessionStore::destroy()` メソッド追加
  - `DestroyError` enum 追加
  - `destroy_session` ハンドラ
- `src/main.rs`: `DELETE /sessions/{session_id}` ルート追加

## 理由

- **明示的な破棄 API**: TTL だけではセッションが残る期間が予測不能であり、viewer が能動的に破棄できる手段が必要
- **viewer_token による認証**: セッションの所有者（viewer）のみが破棄できるべき。publisher がセッションを消すのは不適切
- **204 No Content**: leave API と同様、成功時に返すべきボディがない

## 影響

- viewer がアプリ終了時および serverUrl 変更時にこの API を呼び出す（#55）
- 将来的に TTL ベースの自動削除を追加してもこの API は並存する
