import type { AppConfig } from "../types";
import type { Platform } from "./types";

/**
 * ブラウザ向けの実装（web モードの Connect 専用）
 *
 * - 設定ファイルはない。サーバー URL はビルド時の環境変数 `VITE_NDP_SERVER_URL` で固定する。
 *   それ以外の設定は画面のステートのみで管理し、永続化しない
 * - local モード、Create、モニタウィンドウ、背景画像には対応しない
 *
 * 将来 `localStorage` に対応する場合は、`getConfig` / `saveSetting` の実装を差し替える
 */

/** ビルド時に固定されたサーバー URL（末尾のスラッシュなし。空文字は同一オリジン） */
function serverUrl(): string {
  return (import.meta.env.VITE_NDP_SERVER_URL ?? "").replace(/\/+$/, "");
}

/** ブラウザ版の固定設定 */
function buildConfig(): AppConfig {
  return {
    mode: "web",
    local: { watchDir: "", djId: "" },
    web: { serverUrl: serverUrl() },
    enableComments: false,
    showTags: true,
    eventName: null,
    showEventName: true,
    backgroundImage: null,
    configPath: "",
  };
}

export function createBrowserPlatform(): Platform {
  /** ページ離脱時の退出処理（登録中のみ） */
  let pagehideHandler: (() => void) | null = null;

  const removePagehideHandler = () => {
    if (pagehideHandler) {
      window.removeEventListener("pagehide", pagehideHandler);
      pagehideHandler = null;
    }
  };

  return {
    capabilities: {
      createSession: false,
      configFile: false,
      monitorWindow: false,
      backgroundImage: false,
    },

    windowLabel: "main",

    getConfig: () => Promise.resolve(buildConfig()),
    reloadConfig: () => Promise.resolve(buildConfig()),
    // 永続化しない（画面のステートだけが更新される）
    saveSetting: () => Promise.resolve(),

    // タブを閉じる・リロードするなどでページを離れるときに、セッションから退出する
    // （ベストエフォート。CORS のプリフライトが必要なため、ブラウザによっては届かないことがある）
    setWebSession: (session) => {
      removePagehideHandler();
      pagehideHandler = () => {
        const base = `${session.serverUrl}/api/sessions/${session.sessionId}`;
        const request =
          session.role === "owner"
            ? { url: base, method: "DELETE" }
            : { url: `${base}/disconnect`, method: "POST" };

        fetch(request.url, {
          method: request.method,
          headers: { Authorization: `Bearer ${session.viewerToken}` },
          keepalive: true,
        }).catch(() => {});
      };
      window.addEventListener("pagehide", pagehideHandler);
      return Promise.resolve();
    },
    clearWebSession: () => {
      removePagehideHandler();
      return Promise.resolve();
    },

    // web モードの画像は Data URI のため、変換は不要
    convertFileSrc: (path) => path,
    listBackgroundImages: () => Promise.resolve([]),

    openMonitor: () => Promise.resolve(),
    sendTrackToMonitor: () => {},
    listenMonitorTrack: () => () => {},

    // local モードはブラウザでは使わない
    startLocalDataSource: () => () => {},
  };
}
