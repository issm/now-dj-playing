import { convertFileSrc } from "@tauri-apps/api/core";
import type { TrackPayload } from "./types";

/**
 * モニタウィンドウ用のコンパクト表示コンポーネント
 * アートワークサムネイルと楽曲情報を横並びで表示する
 */
function MonitorView({ track }: { track: TrackPayload | null }) {
    if (!track) {
        return (
            <div className="flex h-screen items-center justify-center bg-black text-gray-400">
                <p className="text-sm">トラック情報を待機中...</p>
            </div>
        );
    }

    const cacheBuster = `?t=${encodeURIComponent(track.updatedAt)}`;
    const artworkSrc = track.artworkPath
        ? convertFileSrc(track.artworkPath) + cacheBuster
        : null;

    return (
        <div className="flex h-screen items-center gap-4 bg-black p-4 text-white">
            {/* アートワークサムネイル */}
            <img
                src={artworkSrc ?? "/default-artwork.png"}
                alt="Artwork"
                className="h-20 w-20 shrink-0 rounded object-cover shadow-md"
                onError={(e) => {
                    e.currentTarget.src = "/default-artwork.png";
                }}
            />

            {/* 楽曲情報 */}
            <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-bold">{track.title}</p>
                <p className="mt-1 truncate text-xs text-gray-300">{track.artist}</p>
                {track.album && (
                    <p className="mt-0.5 truncate text-xs text-gray-500">{track.album}</p>
                )}
            </div>
        </div>
    );
}

export default MonitorView;
