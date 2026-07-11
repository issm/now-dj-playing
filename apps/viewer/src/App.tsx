import { useCallback, useEffect, useRef, useState } from "react";
import { listen, emitTo } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { TrackPayload, AppConfig } from "./types";
import { parseComment, type ParsedComment } from "./commentParser";
import MonitorView from "./MonitorView";

/** success アラートの自動非表示までの時間 (ms) */
const INFO_AUTO_DISMISS_MS = 30_000;

function App() {
    const [track, setTrack] = useState<TrackPayload | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [infoMessage, setInfoMessage] = useState<string | null>(null);
    const [infoDismissing, setInfoDismissing] = useState(false);
    const [showComments, setShowComments] = useState(false);
    const [showTags, setShowTags] = useState(true);
    const [showShortcuts, setShowShortcuts] = useState(false);
    const [reloading, setReloading] = useState(false);
    const [eventName, setEventName] = useState<string | null>(null);
    const [showEventName, setShowEventName] = useState(true);
    const trackRef = useRef<TrackPayload | null>(null);
    const infoTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    const windowLabel = getCurrentWebviewWindow().label;
    const isMonitor = windowLabel === "monitor";

    // info アラートを閉じる（スライドアップアニメーション付き）
    const dismissInfo = useCallback(() => {
        setInfoDismissing(true);
        setTimeout(() => {
            setInfoMessage(null);
            setInfoDismissing(false);
        }, 300); // アニメーション duration に合わせる
    }, []);

    // info メッセージが設定されたら 30 秒後に自動で閉じる
    useEffect(() => {
        if (infoMessage && !infoDismissing) {
            infoTimerRef.current = setTimeout(dismissInfo, INFO_AUTO_DISMISS_MS);
            return () => {
                if (infoTimerRef.current) {
                    clearTimeout(infoTimerRef.current);
                    infoTimerRef.current = null;
                }
            };
        }
    }, [infoMessage, infoDismissing, dismissInfo]);

    // 設定を再読み込みして watcher を起動する
    const handleReloadConfig = async () => {
        setReloading(true);
        setError(null);
        setInfoMessage(null);
        setInfoDismissing(false);
        try {
            const config = await invoke<AppConfig>("reload_config");
            setInfoMessage(`${config.configPath} を読み込みました`);
            setShowComments(config.enableComments);
            setShowTags(config.showTags);
            setEventName(config.eventName);
            setShowEventName(config.showEventName);
            await invoke("start_watch");
        } catch (err) {
            setError(String(err));
        } finally {
            setReloading(false);
        }
    };

    // trackRef を最新の track に追従させる
    useEffect(() => {
        trackRef.current = track;
    }, [track]);

    // キーボードショートカット（メインウィンドウのみ）
    useEffect(() => {
        if (isMonitor) return;

        const handleKeyDown = (e: KeyboardEvent) => {
            switch (e.key) {
                case "c":
                    setShowComments((prev) => !prev);
                    break;
                case "t":
                    setShowTags((prev) => !prev);
                    break;
                case "e":
                    setShowEventName((prev) => !prev);
                    break;
                case "m":
                    invoke("open_monitor")
                        .then(() => {
                            // モニタウィンドウが開いた直後に現在のトラック情報を転送
                            if (trackRef.current) {
                                // 少し待ってからイベントリスナー登録後に送る
                                setTimeout(() => {
                                    emitTo("monitor", "monitor-track", trackRef.current);
                                }, 100);
                            }
                        })
                        .catch((err) => {
                            console.error("モニタウィンドウの起動に失敗:", err);
                        });
                    break;
                case "?":
                    setShowShortcuts((prev) => !prev);
                    break;
                case "Escape":
                    setShowShortcuts(false);
                    break;
            }
        };
        window.addEventListener("keydown", handleKeyDown);
        return () => window.removeEventListener("keydown", handleKeyDown);
    }, [isMonitor]);

    useEffect(() => {
        if (isMonitor) {
            // モニタウィンドウ: メインから転送されたトラック情報を受信
            const unlistenMonitorTrack = listen<TrackPayload>("monitor-track", (event) => {
                setTrack(event.payload);
            });

            return () => {
                unlistenMonitorTrack.then((fn) => fn());
            };
        }

        // メインウィンドウ: バックエンドから設定を取得し、watcher を開始
        let cancelled = false;

        (async () => {
            try {
                const config = await invoke<AppConfig>("get_app_config");

                if (cancelled) return;

                // 設定ファイルのパスを success 表示
                setInfoMessage(`${config.configPath} を読み込みました`);

                // 設定に基づいて初期値を反映
                setShowComments(config.enableComments);
                setShowTags(config.showTags);
                setEventName(config.eventName);
                setShowEventName(config.showEventName);

                // watcher を開始
                await invoke("start_watch");
            } catch (err) {
                if (!cancelled) {
                    setError(String(err));
                }
            }
        })();

        const unlistenTrack = listen<TrackPayload>("track-changed", (event) => {
            setTrack(event.payload);
            setError(null);
            // モニタウィンドウに転送（存在しなくてもエラーにはならない）
            emitTo("monitor", "monitor-track", event.payload);
        });

        const unlistenError = listen<{ dirName: string; message: string }>(
            "watch-error",
            (event) => {
                setError(`${event.payload.dirName}: ${event.payload.message}`);
            },
        );

        return () => {
            cancelled = true;
            unlistenTrack.then((fn) => fn());
            unlistenError.then((fn) => fn());
        };
    }, [isMonitor]);

    // モニタウィンドウの場合はコンパクト表示
    if (isMonitor) {
        return <MonitorView track={track} />;
    }

    return (
        <div className="flex h-screen flex-col items-center justify-center overflow-hidden bg-black text-white">
            {/* success アラート（上部固定、スライドダウン/アップ） */}
            {infoMessage && (
                <div
                    className={`absolute left-0 right-0 top-0 z-40 flex items-center justify-between bg-green-900/80 px-4 py-2 text-sm text-green-200 ${infoDismissing ? "animate-slide-up" : "animate-slide-down"
                        }`}
                >
                    <span>{infoMessage}</span>
                    <button
                        onClick={dismissInfo}
                        className="ml-4 text-green-300 hover:text-white"
                        aria-label="閉じる"
                    >
                        &times;
                    </button>
                </div>
            )}

            {/* error アラート（上部固定、スライドダウン、リロードボタン付き） */}
            {error && (
                <div className="absolute left-0 right-0 top-0 z-50 flex animate-slide-down items-center justify-between bg-red-900/80 px-4 py-2 text-sm text-red-200">
                    <span>{error}</span>
                    <button
                        onClick={handleReloadConfig}
                        disabled={reloading}
                        className="ml-4 shrink-0 rounded bg-red-700 px-3 py-1 text-xs text-red-100 hover:bg-red-600 disabled:opacity-50"
                    >
                        {reloading ? "読込中..." : "再読み込み"}
                    </button>
                </div>
            )}

            {track ? <TrackDisplay track={track} eventName={showEventName ? eventName : null} showComments={showComments} showTags={showTags} /> : <WaitingScreen />}

            {showShortcuts && <ShortcutOverlay onClose={() => setShowShortcuts(false)} />}

            {/* バージョン情報（右下固定） */}
            <VersionDisplay />
        </div>
    );
}

function VersionDisplay() {
    return (
        <div className="absolute bottom-2 right-3 text-xs text-gray-600 select-none">
            Version: {__APP_FULL_VERSION__}
        </div>
    );
}

function ShortcutOverlay({ onClose }: { onClose: () => void }) {
    const shortcuts = [
        { key: "c", description: "コメント表示のトグル" },
        { key: "t", description: "タグ表示のトグル" },
        { key: "e", description: "イベント名表示のトグル" },
        { key: "m", description: "モニタウィンドウを開く" },
        { key: "?", description: "ショートカット一覧の表示" },
        { key: "Esc", description: "オーバーレイを閉じる" },
    ];

    return (
        <div
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/80"
            onClick={onClose}
        >
            <div
                className="w-80 rounded-lg border border-gray-700 bg-gray-900 p-6 shadow-xl"
                onClick={(e) => e.stopPropagation()}
            >
                <h2 className="mb-4 text-lg font-bold text-gray-100">キーボードショートカット</h2>
                <ul className="space-y-3">
                    {shortcuts.map(({ key, description }) => (
                        <li key={key} className="flex items-center justify-between">
                            <span className="text-gray-300">{description}</span>
                            <kbd className="rounded bg-gray-700 px-2 py-0.5 font-mono text-sm text-gray-200">
                                {key}
                            </kbd>
                        </li>
                    ))}
                </ul>
            </div>
        </div>
    );
}

function WaitingScreen() {
    return (
        <div className="text-center">
            <h1 className="text-4xl font-bold">now-dj-playing</h1>
            <p className="mt-4 text-lg text-gray-400">トラック情報を待機中...</p>
        </div>
    );
}

function TrackDisplay({ track, eventName, showComments, showTags }: { track: TrackPayload; eventName: string | null; showComments: boolean; showTags: boolean }) {
    const djDisplay = track.djName ?? track.dirName;
    const cacheBuster = `?t=${encodeURIComponent(track.updatedAt)}`;
    const artworkSrc = track.artworkPath
        ? convertFileSrc(track.artworkPath) + cacheBuster
        : null;
    const djLogoSrc = track.djLogoPath
        ? convertFileSrc(track.djLogoPath) + cacheBuster
        : null;

    return (
        <div className="flex h-full w-full flex-col">
            {/* イベント名 */}
            {eventName && (
                <div className="shrink-0 pt-4 text-center">
                    <span className="text-sm text-gray-400 md:text-base">{eventName}</span>
                </div>
            )}

            {/* ヘッダ: DJ 情報 */}
            <header className="flex h-[100px] shrink-0 items-center justify-center gap-3 px-8">
                {djLogoSrc ? (
                    <img
                        src={djLogoSrc}
                        alt="DJ Logo"
                        className="h-10 w-10 rounded-full object-cover md:h-12 md:w-12"
                    />
                ) : null}
                <span className="text-xl font-semibold text-gray-300 md:text-3xl">
                    {djDisplay}
                </span>
            </header>

            {/* ボディ: アートワーク + 楽曲情報 */}
            <main className="flex min-h-0 flex-1 flex-col items-center justify-center gap-8 px-8 pb-8 md:flex-row md:gap-12">
                {/* 左: アートワーク */}
                <div className="flex w-full shrink-0 items-center justify-center md:h-full md:w-1/2">
                    <img
                        src={artworkSrc ?? "/default-artwork.png"}
                        alt="Artwork"
                        className="w-64 max-h-full rounded-lg object-contain shadow-lg md:w-full md:max-w-[85vh]"
                        onError={(e) => {
                            e.currentTarget.src = "/default-artwork.png";
                        }}
                    />
                </div>

                {/* 右: 楽曲情報 */}
                <div className="flex w-full flex-col items-center justify-center gap-4 md:h-full md:w-1/2 md:items-start">
                    <div className="text-center md:text-left">
                        <h2 className="text-2xl font-bold md:text-4xl">{track.title}</h2>
                        <p className="mt-4 text-lg text-gray-300 md:text-2xl">{track.artist}</p>
                        {track.album && (
                            <p className="mt-4 text-base text-gray-500 md:text-lg">{track.album}</p>
                        )}
                    </div>
                    {showComments && track.comment && (
                        <CommentDisplay raw={track.comment} showTags={showTags} />
                    )}
                </div>
            </main>
        </div>
    );
}

function CommentDisplay({ raw, showTags }: { raw: string; showTags: boolean }) {
    const parsed = parseComment(raw);

    if (!parsed) {
        // 構造化できないコメントはそのまま表示
        return <p className="mt-8 text-base text-gray-400 md:text-lg">{raw}</p>;
    }

    return (
        <div className="mt-8 w-full space-y-2 border-t border-gray-700/50 pt-4">
            {parsed.type === "anison" ? (
                <AnisonCommentView parsed={parsed} showTags={showTags} />
            ) : (
                <GenericCommentView parsed={parsed} showTags={showTags} />
            )}
        </div>
    );
}

function AnisonCommentView({ parsed, showTags }: { parsed: Extract<ParsedComment, { type: "anison" }>; showTags: boolean }) {
    return (
        <>
            {parsed.source && (
                <p className="text-base text-gray-300 md:text-lg">{parsed.source}</p>
            )}
            {parsed.category && (
                <p className="text-base text-gray-500 md:text-lg">{parsed.category}</p>
            )}
            {showTags && (
                <div className="flex flex-wrap gap-2">
                    <span className="rounded bg-indigo-800/60 px-2 py-0.5 text-sm text-indigo-200">
                        anison
                    </span>
                    {parsed.yearTags.map((tag) => (
                        <span
                            key={tag}
                            className="rounded bg-emerald-800/60 px-2 py-0.5 text-sm text-emerald-200"
                        >
                            {tag}
                        </span>
                    ))}
                    {parsed.attrTags.map((tag) => (
                        <span
                            key={tag}
                            className="rounded bg-gray-700/60 px-2 py-0.5 text-sm text-gray-300"
                        >
                            {tag}
                        </span>
                    ))}
                </div>
            )}
        </>
    );
}

function GenericCommentView({ parsed, showTags }: { parsed: Extract<ParsedComment, { type: "generic" }>; showTags: boolean }) {
    return (
        <>
            {parsed.source && (
                <p className="text-base text-gray-300 md:text-lg">{parsed.source}</p>
            )}
            {showTags && (
                <div className="flex flex-wrap gap-2">
                    <span className="rounded bg-purple-800/60 px-2 py-0.5 text-sm text-purple-200">
                        {parsed.primaryTag}
                    </span>
                    {parsed.yearTags.map((tag) => (
                        <span
                            key={tag}
                            className="rounded bg-emerald-800/60 px-2 py-0.5 text-sm text-emerald-200"
                        >
                            {tag}
                        </span>
                    ))}
                    {parsed.attrTags.map((tag) => (
                        <span
                            key={tag}
                            className="rounded bg-gray-700/60 px-2 py-0.5 text-sm text-gray-300"
                        >
                            {tag}
                        </span>
                    ))}
                </div>
            )}
        </>
    );
}

export default App;
