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

        invoke("start_watch", { baseDir }).catch((err) => {
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
        <div className="flex min-h-screen flex-col items-center justify-center bg-black p-8 text-white">
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
    const artworkSrc = track.artworkPath
        ? convertFileSrc(track.artworkPath)
        : null;
    const djLogoSrc = track.djLogoPath
        ? convertFileSrc(track.djLogoPath)
        : null;

    return (
        <div className="flex flex-col items-center gap-6">
            {/* DJ 情報 */}
            <div className="flex items-center gap-3">
                {djLogoSrc ? (
                    <img
                        src={djLogoSrc}
                        alt="DJ Logo"
                        className="h-12 w-12 rounded-full object-cover"
                    />
                ) : null}
                <span className="text-xl font-semibold text-gray-300">{djDisplay}</span>
            </div>

            {/* アートワーク */}
            {artworkSrc ? (
                <img
                    src={artworkSrc}
                    alt="Artwork"
                    className="h-64 w-64 rounded-lg object-cover shadow-lg"
                />
            ) : (
                <div className="flex h-64 w-64 items-center justify-center rounded-lg bg-gray-800">
                    <span className="text-6xl">🎵</span>
                </div>
            )}

            {/* 楽曲情報 */}
            <div className="text-center">
                <h2 className="text-3xl font-bold">{track.title}</h2>
                <p className="mt-2 text-xl text-gray-300">{track.artist}</p>
                {track.album && (
                    <p className="mt-1 text-lg text-gray-500">{track.album}</p>
                )}
            </div>
        </div>
    );
}

export default App;
