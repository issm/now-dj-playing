# ADR-0033: Viewer web モードのセッション管理改善

## ステータス

Accepted

## コンテキスト

viewer の web モードにおいて、以下の問題が発生していた:

1. 設定ファイル読み込み完了後に自動でセッション作成が走る
2. `r` による設定再読み込み後にも新規セッションが作成される
3. 古いセッションはサーバー側で破棄されず、ゾンビセッションが蓄積する

根本原因は `useDataSource` フック内で `config` の変更を検知するたびに `startWebDataSource` が実行され、毎回 `POST /api/sessions/create` を呼んでいたことにある。

## 決定

### 1. セッション作成の明示化

- 設定読み込み後の自動セッション作成を廃止する
- web モード時、画面中央に「Connect」ボタンを表示する
- ユーザーがボタンをクリックした時点で初めてセッションを作成し SSE 接続を開始する
- 接続後は「トラック情報を待機中... ({認証コード})」を表示する

### 2. 設定再読み込み時のセッション維持

- `r` キーによる設定再読み込み時、セッション情報はそのまま維持する
- ただし、再読み込み前後で `web.serverUrl` が変更された場合:
  - 現在の SSE 接続を切断する
  - サーバー側セッションを破棄する（`DELETE /api/sessions/{id}`）
  - 画面を Connect ボタン表示に戻す

### 3. アプリ終了時のセッション破棄

- メインウィンドウ閉鎖時に Tauri の `RunEvent::Exit` を捕捉
- Rust 側で保持しているセッション情報を使い、ブロッキング HTTP で `DELETE /api/sessions/{id}` を送信
- タイムアウト 3 秒のベストエフォート（失敗してもアプリ終了を妨げない）
- publisher-gui の `leave_on_exit` パターンを踏襲

### 実装構成

#### フロントエンド (TypeScript)

- `useDataSource.ts`:
  - `useDataSource` → `useLocalDataSource`（local モード専用）にリネーム
  - `connectWebSession()`: セッション作成 + SSE 接続を行い、cleanup 関数を返す
  - `destroyWebSession()`: サーバーに DELETE を送信
  - `WebSession` 型を export

- `App.tsx`:
  - `WaitingScreen` に Connect ボタンを追加（web モード未接続時のみ表示）
  - `handleConnect`: `connectWebSession` を呼び、`WebSession` を state に保持
  - `handleReloadConfig`: serverUrl 変更検知 → セッション破棄 → Connect ボタンに戻す
  - `set_web_session` IPC で Rust 側にセッション情報を渡す

#### バックエンド (Rust / Tauri)

- `WEB_SESSION` グローバル: フロントから受け取ったセッション情報を保持
- `set_web_session` / `clear_web_session` コマンド
- `destroy_session_on_exit()`: `RunEvent::Exit` 時に `ureq` で DELETE を送信
- `Cargo.toml`: `ureq` 依存を追加

## 理由

- **明示的な Connect**: ユーザーの意図しないセッション作成を防ぎ、ゾンビセッション問題を根本解決する
- **reload 時の維持**: セッションコードを publisher に共有済みの状態で表示設定だけ変えたいケースが多い。serverUrl が変わらない限りセッションを壊す理由がない
- **Rust 側での終了処理**: ブラウザの `beforeunload` は Tauri アプリでは信頼性が低い。Rust の `RunEvent::Exit` フックなら確実に捕捉できる

## 影響

- web モードの UX が「起動 → 即接続」から「起動 → Connect ボタン → 接続」に変わる（1 クリック増）
- サーバー側に `DELETE /api/sessions/{id}` が必要（ADR-0032, #54）
- `ureq` 依存の追加（~150KB、ブロッキング HTTP クライアント）
