import type { AppConfig, BackgroundImageEntry, TrackPayload } from "../types";
import type { DataSourceCallbacks, WebSession } from "../useDataSource";

/** プラットフォームごとの機能の有無（画面側はこれを見て、表示やショートカットを切り替える） */
export interface PlatformCapabilities {
  /** セッションの作成（Create） */
  createSession: boolean;
  /** 設定ファイルの読み込み・再読み込み・書き戻し */
  configFile: boolean;
  /** モニタウィンドウ */
  monitorWindow: boolean;
  /** 背景画像 */
  backgroundImage: boolean;
}

/**
 * Tauri / ブラウザの違いを吸収するインターフェース
 *
 * Tauri API の呼び出しは `tauri.ts` に集約する。画面側（`App.tsx` など）は
 * Tauri API を直接呼ばず、この `Platform` を通す。
 *
 * 実装は `tauri.ts`（デスクトップアプリ）と `browser.ts`（ブラウザ、Connect 専用）。
 */
export interface Platform {
  /** 機能の有無 */
  readonly capabilities: PlatformCapabilities;

  /** 現在のウィンドウのラベル（`main` / `monitor`） */
  readonly windowLabel: string;

  /** アプリ設定を取得する */
  getConfig(): Promise<AppConfig>;
  /** アプリ設定を再読み込みして返す */
  reloadConfig(): Promise<AppConfig>;
  /** 設定値を永続化する（key / value は設定ファイルのキーと JSON 文字列。永続化しない環境では何もしない） */
  saveSetting(key: string, value: string): Promise<void>;

  /** 接続中の web セッションを登録する（アプリ終了時やページ離脱時の破棄 / 退出に使う） */
  setWebSession(session: WebSession): Promise<void>;
  /** 登録した web セッションを解除する */
  clearWebSession(): Promise<void>;

  /** ローカルのファイルパスを `<img src>` などに渡せる URL に変換する */
  convertFileSrc(path: string): string;
  /** 背景画像の一覧を取得する */
  listBackgroundImages(): Promise<BackgroundImageEntry[]>;

  /** モニタウィンドウを開く */
  openMonitor(): Promise<void>;
  /** モニタウィンドウへトラック情報を転送する */
  sendTrackToMonitor(track: TrackPayload): void;
  /** （モニタウィンドウ側で）転送されたトラック情報を購読する。返り値は購読解除の関数 */
  listenMonitorTrack(callback: (track: TrackPayload) => void): () => void;

  /** local モードのデータソースを開始する。返り値は停止の関数 */
  startLocalDataSource(callbacksRef: {
    readonly current: DataSourceCallbacks;
  }): () => void;
}
