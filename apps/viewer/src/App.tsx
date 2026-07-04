import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { TrackPayload } from "./types";

function App() {
    const [track, setTrack] = useState<TrackPayload | null>(null);
    const [error, setError] = useState<string | null>(null);

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

            {track ? <TrackDisplay track={track} /> : <WaitingScreen />}
        </div>
    );
}

function WaitingScreen() {
    return (
        <div className="text-center">
            <h1 className="text-4xl font-bold">Now DJ Playing</h1>
            <p className="mt-4 text-lg text-gray-400">Waiting for track info...</p>
        </div>
    );
}

function TrackDisplay({ track }: { track: TrackPayload }) {
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
                        className="aspect-square w-64 max-h-full rounded-lg object-cover shadow-lg md:w-full md:max-w-[85vh]"
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
                        {import.meta.env.VITE_ENABLE_COMMENTS === "1" && track.comment && (
                            <p className="mt-4 text-base text-gray-400 md:text-lg">{track.comment}</p>
                        )}
                    </div>
                </div>
            </main>
        </div>
    );
}

export default App;
