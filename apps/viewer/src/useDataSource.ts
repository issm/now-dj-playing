import { useEffect, useRef } from "react";
import { platform } from "./platform";
import type { AppConfig, TrackPayload } from "./types";

/** 参加中 DJ の情報 */
export interface DjJoined {
  id: string;
  djName: string;
  djImage: string | null;
}

/** 離脱した DJ の情報 */
export interface DjLeft {
  id: string;
  djName: string;
}

/**
 * web セッションにおける viewer の役割
 *
 * - owner: Create したセッションの作成者。購読とセッション破棄ができる
 * - participant: Connect で参加した viewer。購読のみできる
 */
export type WebSessionRole = "owner" | "participant";

/** owner が他者に配るためのコード */
export interface WebSessionCodes {
  /** publisher 用コード */
  publisher: string;
  /** viewer 用コード（server が古く返さない場合は null） */
  viewer: string | null;
}

/** web セッション情報（接続中に保持） */
export interface WebSession {
  role: WebSessionRole;
  sessionId: string;
  /** owner の token、または参加者用 token（購読に使う） */
  viewerToken: string;
  serverUrl: string;
  /** owner のみ。participant は null */
  codes: WebSessionCodes | null;
  eventName: string | null;
}

/** データソースのコールバック */
export interface DataSourceCallbacks {
  onTrack: (track: TrackPayload) => void;
  onError: (message: string) => void;
  /** web モード: publisher が join したときに呼ばれる */
  onDjJoined?: (dj: DjJoined) => void;
  /** web モード: publisher が leave したときに呼ばれる */
  onDjLeft?: (dj: DjLeft) => void;
  /** web モード: セッションが破棄されたときに呼ばれる（このときすでに SSE は閉じている） */
  onSessionDestroyed?: () => void;
}

/**
 * local モード専用のデータソースフック
 *
 * web モードは createWebSession / connectWebSession / destroyWebSession / disconnectWebSession を使う
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

    return platform.startLocalDataSource(callbacksRef);
  }, [config]);
}

/** エラーからユーザ向けのメッセージを取り出す */
function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** レスポンスのボディ `{ error }` からメッセージを取り出す（なければ fallback） */
async function responseError(resp: Response, fallback: string): Promise<string> {
  try {
    const body = await resp.json();
    if (body && typeof body.error === "string") return body.error;
  } catch {
    // ボディが JSON でない場合は fallback を使う
  }
  return `${fallback}: HTTP ${resp.status}`;
}

/**
 * web セッションの SSE を購読する（Create / Connect 共通）
 *
 * 返り値の cleanup を呼ぶと SSE を切断する（サーバー側セッションは維持）
 *
 * 注意: この関数は Tauri API に依存しない。モニタウィンドウへの転送などは
 * 呼び出し側が onTrack の中で行う
 */
function subscribeToStream(
  session: WebSession,
  callbacks: DataSourceCallbacks,
): () => void {
  const streamUrl = `${session.serverUrl}/api/sessions/${session.sessionId}/stream?token=${session.viewerToken}`;
  const eventSource = new EventSource(streamUrl);
  let closed = false;

  const close = () => {
    closed = true;
    eventSource.close();
  };

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
        djImage: data.dj_image ?? null,
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

  // セッションが破棄された: 自動再接続を繰り返さないよう、先に SSE を閉じる
  eventSource.addEventListener("session_destroyed", () => {
    if (closed) return;
    close();
    console.log(`[web] セッション終了を受信: id=${session.sessionId}`);
    callbacks.onSessionDestroyed?.();
  });

  eventSource.onerror = () => {
    if (!closed) {
      callbacks.onError("サーバとの接続が切断されました");
    }
  };

  return close;
}

/**
 * web モード（Create）: セッションを作成して SSE 接続を開始する
 *
 * 返り値の cleanup を呼ぶと SSE を切断する（サーバー側セッションは維持）
 */
export async function createWebSession(
  config: AppConfig,
  callbacks: DataSourceCallbacks,
  onSessionCreated: (session: WebSession) => void,
): Promise<() => void> {
  const serverUrl = config.web.serverUrl;

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

    console.log(`[web] セッション作成: id=${session.session_id}`);

    // roles を返さない古い server では、従来の code を publisher 用として扱う
    const webSession: WebSession = {
      role: "owner",
      sessionId: session.session_id,
      viewerToken: session.viewer_token,
      serverUrl,
      codes: {
        publisher: session.roles?.publisher?.code ?? session.code,
        viewer: session.roles?.viewer?.code ?? null,
      },
      eventName: session.event_name ?? null,
    };

    onSessionCreated(webSession);

    return subscribeToStream(webSession, callbacks);
  } catch (err) {
    callbacks.onError(errorMessage(err));
    return () => { };
  }
}

/**
 * web モード（Connect）: 作成済みセッションに viewer 用コードで参加し、SSE 接続を開始する
 *
 * 返り値の cleanup を呼ぶと SSE を切断する（参加者用 token の失効は disconnectWebSession で行う）
 */
export async function connectWebSession(
  serverUrl: string,
  code: string,
  callbacks: DataSourceCallbacks,
  onSessionConnected: (session: WebSession) => void,
): Promise<() => void> {
  try {
    const connectResp = await fetch(`${serverUrl}/api/sessions/connect`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code }),
    });

    if (!connectResp.ok) {
      throw new Error(
        await responseError(connectResp, "セッションへの参加に失敗"),
      );
    }

    const session = await connectResp.json();

    console.log(`[web] セッションに参加: id=${session.session_id}`);

    const webSession: WebSession = {
      role: "participant",
      sessionId: session.session_id,
      viewerToken: session.viewer_token,
      serverUrl,
      codes: null,
      eventName: session.event_name ?? null,
    };

    onSessionConnected(webSession);

    return subscribeToStream(webSession, callbacks);
  } catch (err) {
    callbacks.onError(errorMessage(err));
    return () => { };
  }
}

/**
 * web モード: セッションを破棄する（サーバーに DELETE を送信。owner のみ）
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

/**
 * web モード: セッションから退出する（参加者用 token を失効させる。participant のみ）
 *
 * セッション自体は維持される。ベストエフォート: ネットワークエラー時は無視する
 */
export async function disconnectWebSession(session: WebSession): Promise<void> {
  try {
    await fetch(
      `${session.serverUrl}/api/sessions/${session.sessionId}/disconnect`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${session.viewerToken}`,
        },
      },
    );
    console.log(`[web] セッションから退出: id=${session.sessionId}`);
  } catch (err) {
    console.warn("[web] セッションからの退出に失敗（無視）:", err);
  }
}
