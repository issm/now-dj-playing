import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { BackgroundImageEntry } from "./types";

interface BackgroundPickerProps {
    /** 現在選択中の相対パス（null = なし） */
    currentPath: string | null;
    /** 画像選択時のコールバック（null = なし を選択） */
    onSelect: (entry: BackgroundImageEntry | null) => void;
    /** オーバーレイを閉じるコールバック */
    onClose: () => void;
}

/** ウィンドウのアスペクト比を取得する hook */
function useWindowAspectRatio(): number {
    const [ratio, setRatio] = useState(() => window.innerWidth / window.innerHeight);

    useEffect(() => {
        const handleResize = () => {
            setRatio(window.innerWidth / window.innerHeight);
        };
        window.addEventListener("resize", handleResize);
        return () => window.removeEventListener("resize", handleResize);
    }, []);

    return ratio;
}

export default function BackgroundPicker({ currentPath, onSelect, onClose }: BackgroundPickerProps) {
    const [images, setImages] = useState<BackgroundImageEntry[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);
    const aspectRatio = useWindowAspectRatio();

    useEffect(() => {
        (async () => {
            try {
                const entries = await invoke<BackgroundImageEntry[]>("list_background_images");
                setImages(entries);
            } catch (err) {
                setError(String(err));
            } finally {
                setLoading(false);
            }
        })();
    }, []);

    // Escape / b キーで閉じる
    const handleKeyDown = useCallback((e: KeyboardEvent) => {
        if (e.key === "Escape" || e.key === "b") {
            e.stopPropagation();
            onClose();
        }
    }, [onClose]);

    useEffect(() => {
        window.addEventListener("keydown", handleKeyDown);
        return () => window.removeEventListener("keydown", handleKeyDown);
    }, [handleKeyDown]);

    const thumbStyle = { aspectRatio: `${aspectRatio}` };

    return (
        <div
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/80"
            onClick={onClose}
        >
            <div
                className="h-[80vh] w-[80vw] overflow-y-auto rounded-lg border border-gray-700 bg-gray-900 p-6 shadow-xl"
                onClick={(e) => e.stopPropagation()}
            >
                <h2 className="mb-4 text-lg font-bold text-gray-100">背景画像の選択</h2>

                {loading && (
                    <p className="text-gray-400">読み込み中...</p>
                )}

                {error && (
                    <p className="text-red-400">{error}</p>
                )}

                {!loading && !error && (
                    <div className="grid grid-cols-4 gap-3">
                        {/* 「なし」の選択肢 */}
                        <button
                            onClick={() => onSelect(null)}
                            style={thumbStyle}
                            className={`flex w-full cursor-pointer items-center justify-center rounded border-2 text-sm transition-colors ${currentPath === null
                                    ? "border-blue-500 bg-blue-900/30 text-blue-200"
                                    : "border-gray-600 bg-gray-800 text-gray-400 hover:border-gray-500 hover:bg-gray-700"
                                }`}
                        >
                            なし
                        </button>

                        {/* 画像サムネイル */}
                        {images.map((entry) => (
                            <button
                                key={entry.path}
                                onClick={() => onSelect(entry)}
                                style={thumbStyle}
                                className={`group relative w-full cursor-pointer overflow-hidden rounded border-2 transition-colors ${currentPath === entry.path
                                        ? "border-blue-500"
                                        : "border-gray-600 hover:border-gray-500"
                                    }`}
                            >
                                <img
                                    src={convertFileSrc(entry.absolutePath)}
                                    alt={entry.path}
                                    className="h-full w-full object-cover"
                                    loading="lazy"
                                />
                                {/* ファイル名ラベル（ホバー時のみ省略なしで表示） */}
                                <span className="absolute bottom-0 left-0 right-0 truncate bg-black/70 px-1 py-0.5 text-xs text-gray-300 opacity-0 transition-opacity group-hover:whitespace-normal group-hover:break-all group-hover:opacity-100">
                                    {entry.path}
                                </span>
                            </button>
                        ))}
                    </div>
                )}

                {!loading && !error && images.length === 0 && (
                    <p className="mt-2 text-sm text-gray-500">
                        画像ファイルが見つかりません
                    </p>
                )}
            </div>
        </div>
    );
}
