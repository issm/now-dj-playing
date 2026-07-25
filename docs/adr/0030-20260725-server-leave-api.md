# ADR-0030: セッション離脱 API (`POST /sessions/{id}/leave`) の実装

## ステータス

Accepted

## コンテキスト

現状、publisher が `POST /api/sessions/join` でセッションに参加した後、明示的に離脱する手段がない。以下のユースケースで問題となる:

- DJ 交代時に前の DJ がセッションから離脱したことを viewer に通知したい
- セッション終了時に publisher が明示的にクリーンアップしたい
- viewer 側でロースター（参加 DJ 一覧）を正確に保つ必要がある

## 決定

### エンドポイント

```
POST /api/sessions/{session_id}/leave
```

### 認証

publisher の Bearer トークン (`Authorization: Bearer pt_xxxxxxxxxxxx`) で認証する。publish と同じ方式。

### 処理フロー

1. Bearer トークンから publisher を特定
2. パスの `session_id` とトークンに紐づくセッション ID が一致することを検証
3. セッションの `publishers` リストから該当 publisher を削除
4. SSE で `publisher_left` イベントを全 viewer に配信
5. `204 No Content` を返す

### SSE イベント

```
event: publisher_left
data: {"type":"publisher_left","publisher_id":"pub_001","dj_name":"DJ-A"}
```

`publisher_joined` と対称の構造。

### エラーレスポンス

| ステータス | 条件 |
|---|---|
| 401 Unauthorized | Authorization ヘッダが未指定、またはトークンが無効 |
| 403 Forbidden | トークンのセッション ID とパスのセッション ID が不一致 |
| 404 Not Found | セッションまたは publisher が見つからない |

### 実装構成

- `src/leave.rs`: ハンドラ（認証 → `store.leave()` → 204）
- `src/session.rs`: `SessionEvent::PublisherLeft` バリアント、`LeaveError` enum、`SessionStore::leave()` メソッド
- `src/stream.rs`: `event_to_sse` に `PublisherLeft` の変換を追加
- `src/main.rs`: ルート登録

## 理由

- **明示的な離脱 API**: タイムアウトによる暗黙の離脱だけでは、viewer がリアルタイムにロースターを更新できない。明示的な API により即座に `publisher_left` イベントが配信される
- **publish と同じ認証方式**: publisher が既に保持しているトークンをそのまま使えるため、追加の認証フローが不要
- **204 No Content**: リクエスト成功時に返すべきボディがないため、join の 200 + JSON とは異なり 204 を採用

## 影響

- publisher CLI (`ndp-publish`) に leave コマンドまたはオプションを追加する必要がある（別 issue）
- viewer のロースター表示が `publisher_left` イベントで正確に更新される
- セッションファイル (`ndp-publish.session.json`) の扱い（leave 後に削除するか否か）は publisher 側の実装で決定する
