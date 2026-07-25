import { useEffect, useRef } from "react";
import { listen, emitTo } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, TrackPayload } from "./types";

/** 参加中 DJ の情報 */
export interface DjJoined {
  id: string;
  djName: string;
}

/** 離脱した DJ の情報 */
export interface DjLeft {
  id: string;
  djName: string;
}

/** web セッション情報（接続中に保持） */
export interface WebSession {
  sessionId: string;
  code: string;
  viewerToken: string;
  serverUrl: string;
}

/** データソースのコールバック */
export interface DataSourceCallbacks {
  onTrack: (track: TrackPayload) => void;
  onError: (message: string) => void;
  /** web モード: publisher が join したときに呼ばれる */
  onDjJoined?: (dj: DjJoined) => void;
  /** web モード: publisher が leave したときに呼ばれる */
  onDjLeft?: (dj: DjLeft) => void;
}

/**
 * local モード専用のデータソースフック
 *
 * web モードは connectWebSession / disconnectWebSession を使う
 */
export function useLocalDataSource(
  config: AppConfig | null,
  callbacks: DataSourceCallbacks,
) {
  const callbacksRef = useRef(callbacks);
  callbacksRef.current = callbacks;

  useEffect(() => {
    if (!config) return;
    if (config.mode !== "local") return;

    return startLocalDataSource(config, callbacksRef);
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

/**
 * web モード: セッションを作成して SSE 接続を開始する
 *
 * 返り値の cleanup を呼ぶと SSE を切断する（サーバー側セッションは維持）
 */
export async function connectWebSession(
  config: AppConfig,
  callbacks: DataSourceCallbacks,
  onSessionCreated: (session: WebSession) => void,
): Promise<() => void> {
  const serverUrl = config.web.serverUrl;
  let eventSource: EventSource | null = null;
  let cancelled = false;

  const cleanup = () => {
    cancelled = true;
    if (eventSource) {
      eventSource.close();
      eventSource = null;
    }
  };

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
    if (cancelled) return cleanup;

    console.log(
      `[web] セッション作成: id=${session.session_id}, code=${session.code}`,
    );

    const webSession: WebSession = {
      sessionId: session.session_id,
      code: session.code,
      viewerToken: session.viewer_token,
      serverUrl,
    };

    onSessionCreated(webSession);

    // SSE 接続
    const streamUrl = `${serverUrl}/api/sessions/${session.session_id}/stream?token=${session.viewer_token}`;
    eventSource = new EventSource(streamUrl);

    eventSource.addEventListener("track_changed", (event) => {
      try {
        const data = JSON.parse(event.data);
        const track: TrackPayload = {
          dirName: data.publisher_id,
          djName: data.dj_name,
          djLogoPath: null,
          title: data.title,
          artist: data.artist,
          album: data.album ?? null,
          comment: data.comment ?? null,
          artworkPath: data.artwork ?? null,
          updatedAt: data.updated_at,
        };
        callbacks.onTrack(track);
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
        callbacks.onDjJoined?.({
          id: data.publisher_id,
          djName: data.dj_name,
        });
      } catch {
        // 無視
      }
    });

    eventSource.addEventListener("publisher_left", (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log(
          `[web] publisher 離脱: ${data.dj_name} (${data.publisher_id})`,
        );
        callbacks.onDjLeft?.({
          id: data.publisher_id,
          djName: data.dj_name,
        });
      } catch {
        // 無視
      }
    });

    eventSource.onerror = () => {
      if (!cancelled) {
        callbacks.onError("サーバとの接続が切断されました");
      }
    };
  } catch (err) {
    if (!cancelled) {
      callbacks.onError(String(err));
    }
  }

  return cleanup;
}

/**
 * web モード: セッションを破棄する（サーバーに DELETE を送信）
 *
 * ベストエフォート: ネットワークエラー時は無視する
 */
export async function destroyWebSession(session: WebSession): Promise<void> {
  try {
    await fetch(`${session.serverUrl}/api/sessions/${session.sessionId}`, {
      method: "DELETE",
      headers: {
        Authorization: `Bearer ${session.viewerToken}`,
      },
    });
    console.log(`[web] セッション破棄: id=${session.sessionId}`);
  } catch (err) {
    console.warn("[web] セッション破棄に失敗（無視）:", err);
  }
}
