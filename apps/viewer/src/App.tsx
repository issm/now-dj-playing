import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { TrackPayload } from "./types";
import { parseComment, type ParsedComment } from "./commentParser";

function App() {
    const [track, setTrack] = useState<TrackPayload | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [showComments, setShowComments] = useState(
        import.meta.env.VITE_ENABLE_COMMENTS === "1",
    );

    // `c` キーでコメント表示をトグル
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === "c") {
                setShowComments((prev) => !prev);
            }
        };
        window.addEventListener("keydown", handleKeyDown);
        return () => window.removeEventListener("keydown", handleKeyDown);
    }, []);

    useEffect(() => {
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
    }, []);

    return (
        <div className="flex h-screen flex-col items-center justify-center overflow-hidden bg-black text-white">
            {error && (
                <div className="mb-4 rounded bg-red-900/50 px-4 py-2 text-red-200">
                    {error}
                </div>
            )}

            {track ? <TrackDisplay track={track} showComments={showComments} /> : <WaitingScreen />}
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
