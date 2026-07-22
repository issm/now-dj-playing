import { useEffect, useRef } from "react";
import { listen, emitTo } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, TrackPayload } from "./types";

/** データソースのコールバック */
export interface DataSourceCallbacks {
  onTrack: (track: TrackPayload) => void;
  onError: (message: string) => void;
  /** web モード: セッション作成完了時に呼ばれる */
  onSessionCreated?: (sessionCode: string) => void;
}

/**
 * モードに応じたデータソースを開始するカスタムフック
 *
 * - local: Tauri IPC で track-changed イベントを listen
 * - web: ndp-server に SSE 接続して track_changed を受信
 */
export function useDataSource(
  config: AppConfig | null,
  callbacks: DataSourceCallbacks,
) {
  const callbacksRef = useRef(callbacks);
  callbacksRef.current = callbacks;

  useEffect(() => {
    if (!config) return;

    if (config.mode === "local") {
      return startLocalDataSource(config, callbacksRef);
    } else if (config.mode === "web") {
      return startWebDataSource(config, callbacksRef);
    }
  }, [config]);
}

/** local モード: Tauri IPC */
function startLocalDataSource(
  _config: AppConfig,
  callbacksRef: React.RefObject<DataSourceCallbacks>,
): () => void {
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
}

/** web モード: ndp-server に SSE 接続 */
function startWebDataSource(
  config: AppConfig,
  callbacksRef: React.RefObject<DataSourceCallbacks>,
): () => void {
  const serverUrl = config.web.serverUrl;
  let eventSource: EventSource | null = null;
  let cancelled = false;

  (async () => {
    try {
      // セッション作成
      const createResp = await fetch(`${serverUrl}/api/sessions/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ event_name: config.eventName }),
      });

      if (!createResp.ok) {
        throw new Error(`セッション作成に失敗: HTTP ${createResp.status}`);
      }

      const session = await createResp.json();
      if (cancelled) return;

      console.log(
        `[web] セッション作成: id=${session.session_id}, code=${session.code}`,
      );

      // セッションコードを通知
      callbacksRef.current.onSessionCreated?.(session.code);

      // SSE 接続
      // EventSource は Authorization ヘッダを送れないため、クエリパラメータでトークンを渡す
      const streamUrl = `${serverUrl}/api/sessions/${session.session_id}/stream?token=${session.viewer_token}`;
      eventSource = new EventSource(streamUrl);

      eventSource.addEventListener("track_changed", (event) => {
        try {
          const data = JSON.parse(event.data);
          // ndp-server の TrackData → TrackPayload に変換
          const track: TrackPayload = {
            dirName: data.publisher_id,
            djName: data.dj_name,
            djLogoPath: null,
            title: data.title,
            artist: data.artist,
            album: data.album ?? null,
            comment: data.comment ?? null,
            // ndp-server から Base64 Data URI (data:image/...) がそのまま送られてくる
            artworkPath: data.artwork ?? null,
            updatedAt: data.updated_at,
          };
          callbacksRef.current.onTrack(track);
          // モニタウィンドウに転送
          emitTo("monitor", "monitor-track", track);
        } catch (e) {
          console.error("[web] track_changed パースエラー:", e);
        }
      });

      eventSource.addEventListener("publisher_joined", (event) => {
        try {
          const data = JSON.parse(event.data);
          console.log(
            `[web] publisher 参加: ${data.dj_name} (${data.publisher_id})`,
          );
        } catch {
          // 無視
        }
      });

      eventSource.onerror = () => {
        if (!cancelled) {
          callbacksRef.current.onError("サーバとの接続が切断されました");
        }
      };
    } catch (err) {
      if (!cancelled) {
        callbacksRef.current.onError(String(err));
      }
    }
  })();

  return () => {
    cancelled = true;
    if (eventSource) {
      eventSource.close();
    }
  };
}
