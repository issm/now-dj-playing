import { convertFileSrc } from "@tauri-apps/api/core";
import type { TrackPayload } from "./types";

/**
 * モニタウィンドウ用のコンパクト表示コンポーネント
 * アートワークを上部に、曲名・アーティストを下部に縦並びで表示する
 */
function MonitorView({ track }: { track: TrackPayload | null }) {
    if (!track) {
        return (
            <div className="flex h-screen items-center justify-center bg-black text-gray-400">
                <p className="text-xs">トラック情報を待機中...</p>
            </div>
        );
    }

    const cacheBuster = `?t=${encodeURIComponent(track.updatedAt)}`;
    const artworkSrc = track.artworkPath
        ? convertFileSrc(track.artworkPath) + cacheBuster
        : null;

    return (
        <div className="flex h-screen flex-col bg-black p-3 text-white">
            {/* アートワーク */}
            <div className="flex shrink-0 justify-center">
                <img
                    src={artworkSrc ?? "/default-artwork.png"}
                    alt="Artwork"
                    className="aspect-square w-3/4 rounded object-cover shadow-md"
                    onError={(e) => {
                        e.currentTarget.src = "/default-artwork.png";
                    }}
                />
            </div>

            {/* 楽曲情報: 曲名 + アーティストのみ */}
            <div className="mt-2 min-w-0">
                <p className="truncate text-sm font-bold">{track.title}</p>
                <p className="mt-0.5 truncate text-xs text-gray-300">{track.artist}</p>
            </div>
        </div>
    );
}

export default MonitorView;
