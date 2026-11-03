# ADR-0037: viewer の稼働中セッションへの参加 (Connect)

## ステータス

Accepted

## コンテキスト

viewer の web モードは、セッションを作成する側 (Create) にしか対応していない。稼働中のセッションに後から参加 (Connect) して閲覧できるようにし、複数箇所での同時表示を可能にする (#80)。

server 側の API と権限モデルは ADR-0036 (#82) で追加済み (ndp-server 0.4.0)。本 ADR は viewer 側の対応を定める。

| 権限 | viewer での位置づけ | コード |
|---|---|---|
| owner | Create した viewer。購読とセッション破棄ができる | 不要 (token を持つ) |
| view | Connect した viewer。購読のみ。`disconnect` で離脱する | `roles.viewer.code` |
| publish | publisher (viewer は扱わない) | `roles.publisher.code` |

用語は ADR-0033 の整理に従う。Create は新規作成して SSE につなぐこと、Connect は作成済みセッションに view 権限で参加して SSE につなぐこと。

## 決定

### 1. 待機画面

web モードで未接続のとき、ヘッダに次を並べる。

- 「Create」ボタン (従来どおり)
- 6 桁のコード入力欄と「Connect」ボタン
  - 入力は数字のみ受け付ける。6 桁そろうまで Connect は押せない。Enter でも送信できる
  - `POST /api/sessions/connect` を呼び、得た参加者用 token で SSE に接続する
  - 無効なコードなどのエラーは、既存のエラーアラートに表示する

### 2. セッションの役割 (`WebSession.role`)

`WebSession` に `role` (`"owner"` / `"participant"`) を持たせ、離脱時の処理を分ける。

| 操作 | owner (Create) | participant (Connect) |
|---|---|---|
| `Shift+S` | セッションを破棄 (`DELETE`) | セッションから退出 (SSE 切断と `POST /disconnect`) |
| アプリ終了 | セッションを破棄 | `POST /disconnect` |
| `serverUrl` 変更を伴う設定再読み込み | セッションを破棄 | セッションから退出 |

- いずれも best-effort とし、失敗してもアプリの動作を妨げない
- participant は破棄 (`DELETE`) を一切呼ばない。呼んでも server が 403 を返す
- Issue #80 の当初案は「`Shift+S` を participant では無効にする」だったが、退出手段がなくなり別のセッションにつなぎ直せないため、`Shift+S` は participant では「退出」として動作させる。ショートカット一覧の表記は「セッションの破棄 / 退出」とする

### 3. アプリ終了時の処理 (Rust)

`set_web_session` に省略可能な `role` を追加する。

- 省略時は owner として扱う (従来の呼び出しと互換)
- 終了時、owner は `DELETE /api/sessions/{id}`、participant は `POST /api/sessions/{id}/disconnect` を、タイムアウト 3 秒のブロッキング HTTP (`ureq`) で送る
- Connect した場合も `set_web_session` を `role: "participant"` で呼ぶ

### 4. 認証コードの表示 (Create 側)

Create のレスポンスの `roles.publisher.code` と `roles.viewer.code` を、ラベル付きで並べて表示する。

```
publish 587831   view 482910
```

- 配置は従来の認証コードの位置 (ヘッダ右端、ロスターが空のときは中央) のまま、コードを 2 つに増やす
- ラベルとコードはマウス選択できない。コードをクリックするとクリップボードにコピーする (カーソルは pointer。コピーできたことは、コードの色が一瞬変わることで示す)
- server が 0.4.0 より古く `roles` を返さない場合は、`code` を publish 用として表示し、view 用は表示しない
- Connect した viewer (participant) には、コードを表示しない

### 5. `session_destroyed` と接続の扱い

- `session_destroyed` を受信したら、`EventSource` を即座に閉じる。サーバー側でストリームが閉じると `EventSource` は自動再接続を繰り返し、エラーアラートが出続けてしまうため
- 受信後は、状態をリセットして待機画面に戻し、「セッションが終了しました」を情報アラートで表示する
- この経路ではエラーアラート (「サーバとの接続が切断されました」) を出さない
- Create 側が自分で破棄する場合は、SSE を先に閉じるので、このイベントは受信しない (従来どおり)

### 6. イベント名

Connect のレスポンスの `event_name` が空でなければ、イベント名として表示する。Create 側がセッション作成後に `e` で編集した内容は server に送られないため、参加側には伝わらない (既知の制限)。

### 7. 表示設定

コメント、タグ、イベント名の表示、背景画像などの表示設定は、各 viewer が自分の設定として持つ。Connect しても他の viewer の設定は変わらない。

### 実装構成

- `useDataSource.ts`:
  - SSE の購読処理を共通関数 (`subscribeToStream`) に切り出し、`createWebSession` / `connectWebSession` から使う
  - `connectWebSession()` を追加 (`POST /connect` → 共通の購読処理)
  - `disconnectWebSession()` を追加 (`POST /disconnect`)
  - `WebSession` に `role`、`codes`、`eventName` を追加。従来の `code` は `codes.publisher` に置き換える
  - `DataSourceCallbacks` に `onSessionDestroyed` を追加
  - SSE の購読処理から、モニタウィンドウへの転送 (`emitTo`) を取り除く。Tauri API への依存を、購読処理から外すため。転送は呼び出し側 (`App.tsx`) のコールバックで行う
- `App.tsx`:
  - `handleConnect` と、コード入力欄の state を追加
  - Create と Connect のコールバックを共通化する
  - 離脱処理 (破棄 / 退出) を、`role` に応じて分岐する共通関数にまとめる
  - ショートカットのハンドラは、`input` / `textarea` にフォーカスがあるときは何もしない (コード入力中の文字が、`r` `c` `t` などのショートカットに誤って反応しないように)
  - 認証コード表示を 2 つに増やす
- `src-tauri/src/lib.rs`: `set_web_session` に `role` を追加し、終了時の処理を分岐する
- `README.md` (`apps/viewer`): Connect の説明、ショートカットの表記を更新する

## 理由

- **離脱処理を `role` で分ける**: participant が `DELETE` を呼ぶと必ず 403 になるため、役割に合った API を呼ぶ。owner が破棄すれば全員が終了するため、participant の退出とは意味が違う
- **`Shift+S` を participant では退出にする**: 無効にすると、別のセッションにつなぎ直すにはアプリの再起動が必要になる。退出は破棄ではなく、他の viewer や publisher に影響しない
- **終了時の `disconnect` を Rust で送る**: Create 側の破棄と同じ理由で、ブラウザの終了イベントに頼らず、`RunEvent::Exit` で確実に送る。送らないと、参加者用 token がセッション破棄まで残る (失効の自動化は #81 で扱う)
- **`session_destroyed` で `EventSource` を閉じる**: 閉じないと、自動再接続がエラーを返し続ける。ADR-0036 で追加したイベントの、viewer 側の受け口にあたる
- **購読処理から Tauri API を外す**: ブラウザ対応 (#77) で `Platform` 抽象に移すときの手戻りを減らすため。Connect の経路は `fetch` と `EventSource` だけで動く
- **コードを 2 つ表示する**: Create した viewer が、publisher と、閲覧のみの参加者に、それぞれ別のコードを配れるようにするため

## 影響

- web モードの待機画面に、コード入力欄と Connect ボタンが増える
- Create 側の認証コード表示が 2 つになる。レイアウトの変更は最小限とする
- server 0.4.0 以降が前提。それより古い server に対しては、Create は従来どおり動き (view 用コードは表示されない)、Connect は失敗する
- ブラウザ対応 (#77) では、ブラウザ版を Connect 専用にできる。その場合、破棄の扱い (`pagehide` や TTL) は論点から外れる
- コードの総当たり対策、参加者数の制限、異常終了した参加者 token の失効は #81 で扱う
- 参加側のイベント名は、Create 側が後から編集した内容に追従しない
