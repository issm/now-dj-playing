import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { AppConfig, BackgroundImageEntry, TrackPayload } from "../types";
import type { Platform } from "./types";

/**
 * Tauri（デスクトップアプリ）向けの実装
 *
 * Tauri API の呼び出しはすべてここに集約する。
 */
export function createTauriPlatform(): Platform {
  return {
    windowLabel: getCurrentWebviewWindow().label,

    getConfig: () => invoke<AppConfig>("get_app_config"),
    reloadConfig: () => invoke<AppConfig>("reload_config"),
    saveSetting: (key, value) => invoke<void>("update_config_value", { key, value }),

    setWebSession: (session) =>
      invoke<void>("set_web_session", {
        sessionId: session.sessionId,
        viewerToken: session.viewerToken,
        serverUrl: session.serverUrl,
        role: session.role,
      }),
    clearWebSession: () => invoke<void>("clear_web_session"),

    convertFileSrc: (path) => convertFileSrc(path),
    listBackgroundImages: () =>
      invoke<BackgroundImageEntry[]>("list_background_images"),

    openMonitor: () => invoke<void>("open_monitor"),
    sendTrackToMonitor: (track) => {
      emitTo("monitor", "monitor-track", track);
    },
    listenMonitorTrack: (callback) => {
      const unlisten = listen<TrackPayload>("monitor-track", (event) => {
        callback(event.payload);
      });
      return () => {
        unlisten.then((fn) => fn());
      };
    },

    // local モード: Tauri IPC
    startLocalDataSource: (callbacksRef) => {
      // watcher を開始
      invoke("start_watch").catch((err) => {
        callbacksRef.current.onError(String(err));
      });

      const unlistenTrack = listen<TrackPayload>("track-changed", (event) => {
        callbacksRef.current.onTrack(event.payload);
        // モニタウィンドウに転送
        emitTo("monitor", "monitor-track", event.payload);
      });

      const unlistenError = listen<{ dirName: string; message: string }>(
        "watch-error",
        (event) => {
          callbacksRef.current.onError(
            `${event.payload.dirName}: ${event.payload.message}`,
          );
        },
      );

      return () => {
        unlistenTrack.then((fn) => fn());
        unlistenError.then((fn) => fn());
      };
    },
  };
}
