# ADR-0036: viewer の参加 (connect / disconnect) API と権限モデル

## ステータス

Accepted

## コンテキスト

現状の viewer は、セッションを作成する側 (`POST /sessions/create`) にしか対応していない。稼働中のセッションに複数の viewer が参加し、複数箇所で同時に表示したい (#80)。server 側には次の課題がある (#82)。

1. **2 つ目の viewer に認証手段がない**: `viewer_token` は `create` のレスポンスでしか返らない。この token は購読 (SSE) と破棄 (`DELETE`) の両方の権限を持つため、そのまま共有すると参加者がセッションを破棄できてしまう
2. **途中参加で状態が欠ける**: SSE 接続直後に送られるのは `last_track` のみ。先に参加した DJ のロスターは `publisher_joined` を受け取っていないと分からない
3. **セッション破棄が参加側に伝わらない**: 破棄すると broadcast チャネルが消えてストリームが黙って終わり、`EventSource` は自動再接続を繰り返す

## 決定

### 1. 権限モデル (3 種類)

| 権限 | できること | 渡し方 | コード |
|---|---|---|---|
| owner | 購読、セッション破棄 | `create` のレスポンスで token (`vt_...`) を受け取る | 不要 |
| publish | 楽曲の送信、`leave` | 6 桁コードで `join` し、publisher token を受け取る | 既存の `code` |
| view | 購読、`disconnect` | 6 桁コードで `connect` し、参加者用 token を受け取る | 新規の view 用コード |

- owner 権限は create 主だけが持つ前提なので、owner 用のコードは発行しない
- 閲覧用に配ったコードで publisher として `join` できてしまわないよう、publish 用と view 用のコードは別の値で発行する。互いのエンドポイントに入れると無効コードとして 404 を返す
- 参加者用 token は `connect` の呼び出しごとに発行する
- 既存の `code` は publish 用のまま変更しない (デプロイ済みの publisher との互換)

### 2. `create` のレスポンス

既存のフィールドは変更せず、`roles` を追加する。リクエストの形は変えない。

```json
{
  "session_id": "<UUID v4>",
  "code": "037482",
  "event_name": null,
  "viewer_token": "vt_<UUID>",
  "roles": {
    "publisher": { "code": "037482" },
    "viewer":    { "code": "482910" }
  }
}
```

- 新しいクライアントは `roles` だけを見ればよい
- `roles.publisher.code` はトップレベルの `code` と同じ値
- トップレベルの `viewer_token` は、名前に反して **owner の token** である。過去互換のために名前を残しており、`roles.viewer` とは無関係
- 各ロールの値をオブジェクトにしておくことで、将来 `expires_at` などを足しても既存クライアントを壊さない
- `roles` には token を含めない。`join` / `connect` はコードを知っていれば呼べるため

### 3. `POST /api/sessions/connect`

- リクエスト: `{ "code": "<view 用コード>" }`
- レスポンス (200): `{ "session_id", "viewer_token", "event_name" }`
  - フィールド名は既存クライアントとの一貫性のため `viewer_token` とする。値は参加者用 token
- 無効なコードは 404 (`join` と同じ)

### 4. `POST /api/sessions/{id}/disconnect`

- 認証: 参加者用 token (`Authorization: Bearer`)
- 成功時は 204。その token を一覧から削除し、以降その token での購読は 401 になる
- エラー: Authorization ヘッダなしは 401、セッションなしは 404、参加者用 token として登録されていない値 (owner の token、別セッションの token を含む) は 401

### 5. 購読と破棄の認可

- `GET /api/sessions/{id}/stream`: owner の token、または参加者用 token のどちらでも購読できる。別セッションの token は従来どおり 403
- `DELETE /api/sessions/{id}`: owner の token のみ許可する。参加者用 token は 403 (従来の token 一致判定のままで満たされる)

### 6. SSE 接続時のスナップショット

接続直後に、現在の publisher 全員分の `publisher_joined` を送り、その後に `last_track` を送る。

- 途中参加でもロスターが復元される
- 通信断のあとの `EventSource` 自動再接続でも復元される
- subscribe を先に行ってから publisher 一覧を読むため、取りこぼしはない。その代わり、参加の瞬間とスナップショットが重なると `publisher_joined` が重複して届き得る。viewer のハンドラは冪等 (同じ名前と画像なら state を更新しない)

### 7. `session_destroyed` イベント

破棄の直前に、購読中の全 viewer に `session_destroyed` を broadcast する。参加側はこれを受けて再接続せず、終了を表示できる。

- データ: `{ "type": "session_destroyed" }`
- 既存 viewer は、名前付きイベントごとに `addEventListener` で受けているため、未登録のこのイベントは無視される

### 実装構成

- `src/session.rs`:
  - `Session` に `viewer_code` と `connected_viewers` を追加
  - `SessionEvent::SessionDestroyed` を追加
  - `SessionStore::connect_by_code()` / `disconnect()` / `get_publishers()` を追加
  - `find_session_by_viewer_token()` を owner と参加者用 token の両方に対応させる
  - `destroy()` で `session_destroyed` を broadcast してからチャネルを削除する
  - `create` のレスポンスに `roles` を追加
- `src/connect.rs` (新規): `connect` / `disconnect` ハンドラ
- `src/stream.rs`: スナップショット送信と `session_destroyed` の変換
- `src/main.rs`: ルート追加

## 理由

- **token の権限分離**: owner の token を共有すると、参加者が破棄できてしまう。購読のみ可能な token を別に発行することで、権限を最小にできる
- **コードの分離**: 閲覧用に配るコードは、楽曲を送れないようにしたい。会場のスクリーンや観客など、閲覧のみの相手を想定する
- **owner 用コードを作らない**: owner を他者に渡す用途がなく、発行する理由がない
- **`roles` の追加方式**: 既存フィールドを変えないので、デプロイ済みのクライアントが壊れない。リクエストにパラメータを足す案もあったが、`viewer_code` を常に発行する単純さを優先した
- **スナップショット**: ロスターの復元は connect の前提機能であり、通信断後の再接続にも効く

## 影響

- 既存クライアント (publisher、publisher-gui、現行の viewer) への影響はない。`create` はフィールドの追加のみ、`session_destroyed` は未登録イベントとして無視される。現行の viewer が再接続したときは、ロスターが復元される
- viewer 側の対応 (#80) は、本変更のデプロイ後に行う
- コードの総当たり対策、参加者数の制限、異常終了した参加者 token の失効、セッション TTL は #81 で扱う
- SSE の切断中に publisher が `leave` した場合、既存 viewer のロスターから消えない。スナップショットは現在の全員を表すだけで、消えた側は判定できないため。現行の挙動で、本変更では扱わない
- create 主が編集したイベント名は、サーバに送られないため参加側に伝わらない。参加側は `connect` 時点の値を表示する
