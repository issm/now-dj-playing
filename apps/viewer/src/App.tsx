import { useEffect, useRef, useState } from "react";
import { listen, emitTo } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { TrackPayload } from "./types";
import { parseComment, type ParsedComment } from "./commentParser";
import MonitorView from "./MonitorView";

function App() {
    const [track, setTrack] = useState<TrackPayload | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [showComments, setShowComments] = useState(
        import.meta.env.VITE_ENABLE_COMMENTS === "1",
    );
    const [showShortcuts, setShowShortcuts] = useState(false);
    const trackRef = useRef<TrackPayload | null>(null);

    const windowLabel = getCurrentWebviewWindow().label;
    const isMonitor = windowLabel === "monitor";

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

        // メインウィンドウ: watcher を開始し、トラック情報をモニタに転送
        const baseDir = import.meta.env.VITE_WATCH_DIR;

        if (!baseDir) {
            setError("VITE_WATCH_DIR が設定されていません。.env.development を確認してください。");
            return;
        }

        const djId = import.meta.env.VITE_DEFAULT_DJ_ID || "dj-000";

        invoke("start_watch", { baseDir, djId }).catch((err) => {
            setError(String(err));
        });

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
            {error && (
                <div className="mb-4 rounded bg-red-900/50 px-4 py-2 text-red-200">
                    {error}
                </div>
            )}

            {track ? <TrackDisplay track={track} showComments={showComments} /> : <WaitingScreen />}

            {showShortcuts && <ShortcutOverlay onClose={() => setShowShortcuts(false)} />}
        </div>
    );
}

function ShortcutOverlay({ onClose }: { onClose: () => void }) {
    const shortcuts = [
        { key: "c", description: "コメント表示のトグル" },
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

function TrackDisplay({ track, showComments }: { track: TrackPayload; showComments: boolean }) {
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
            {/* ヘッダ: DJ 情報 */}
            <header className="flex h-[15vh] shrink-0 items-center justify-center gap-3 px-8">
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
                        <CommentDisplay raw={track.comment} />
                    )}
                </div>
            </main>
        </div>
    );
}

function CommentDisplay({ raw }: { raw: string }) {
    const parsed = parseComment(raw);

    if (!parsed) {
        // 構造化できないコメントはそのまま表示
        return <p className="mt-8 text-base text-gray-400 md:text-lg">{raw}</p>;
    }

    return (
        <div className="mt-8 w-full space-y-2 border-t border-gray-700/50 pt-4">
            {parsed.type === "anison" ? (
                <AnisonCommentView parsed={parsed} />
            ) : (
                <GenericCommentView parsed={parsed} />
            )}
        </div>
    );
}

function AnisonCommentView({ parsed }: { parsed: Extract<ParsedComment, { type: "anison" }> }) {
    return (
        <>
            <div>
                <span className="rounded bg-indigo-800/60 px-2 py-0.5 text-sm text-indigo-200">
                    anison
                </span>
            </div>
            {parsed.source && (
                <p className="text-base text-gray-300 md:text-lg">{parsed.source}</p>
            )}
            {parsed.category && (
                <p className="text-base text-gray-500 md:text-lg">{parsed.category}</p>
            )}
            <div className="flex flex-wrap gap-2">
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
        </>
    );
}

function GenericCommentView({ parsed }: { parsed: Extract<ParsedComment, { type: "generic" }> }) {
    return (
        <>
            {parsed.source && (
                <p className="text-base text-gray-300 md:text-lg">{parsed.source}</p>
            )}
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
        </>
    );
}

export default App;
