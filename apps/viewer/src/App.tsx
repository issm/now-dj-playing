import { useCallback, useEffect, useRef, useState } from "react";
import type { TrackPayload, AppConfig, BackgroundImageEntry, BackgroundImageConfig } from "./types";
import { parseComment, type ParsedComment } from "./commentParser";
import {
    useLocalDataSource,
    createWebSession,
    connectWebSession,
    destroyWebSession,
    disconnectWebSession,
    type DataSourceCallbacks,
    type WebSession,
    type WebSessionCodes,
} from "./useDataSource";
import { platform } from "./platform";
import { resolveImageSrc } from "./artwork";
import MonitorView from "./MonitorView";
import BackgroundPicker from "./BackgroundPicker";

/** success アラートの自動非表示までの時間 (ms) */
const INFO_AUTO_DISMISS_MS = 10_000;

/** セッションコードの桁数 */
const SESSION_CODE_LENGTH = 6;

function App() {
    const [track, setTrack] = useState<TrackPayload | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [infoMessage, setInfoMessage] = useState<string | null>(null);
    const [infoDismissing, setInfoDismissing] = useState(false);
    const [showComments, setShowComments] = useState(false);
    const [showTags, setShowTags] = useState(true);
    const [showShortcuts, setShowShortcuts] = useState(false);
    const [showBackgroundPicker, setShowBackgroundPicker] = useState(false);
    const [reloading, setReloading] = useState(false);
    const [eventName, setEventName] = useState<string | null>(null);
    const [showEventName, setShowEventName] = useState(true);
    /** イベント名編集中フラグ */
    const [editingEventName, setEditingEventName] = useState(false);
    /** イベント名編集中の入力値 */
    const [editingEventNameValue, setEditingEventNameValue] = useState("");
    /** 背景画像設定（null = 機能無効） */
    const [backgroundImageConfig, setBackgroundImageConfig] = useState<BackgroundImageConfig | null>(null);
    /** 現在選択中の背景画像の相対パス（null = なし） */
    const [backgroundImagePath, setBackgroundImagePath] = useState<string | null>(null);
    /** 背景画像の絶対パス（表示用、null = なし） */
    const [backgroundImageAbsolutePath, setBackgroundImageAbsolutePath] = useState<string | null>(null);
    const trackRef = useRef<TrackPayload | null>(null);
    const infoTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    const windowLabel = platform.windowLabel;
    const isMonitor = windowLabel === "monitor";

    /** 設定（データソースフックに渡す） */
    const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
    /** web モードの認証コード（Create した viewer のみ。Connect した viewer は null） */
    const [sessionCodes, setSessionCodes] = useState<WebSessionCodes | null>(null);
    /** web セッション情報（接続中に保持） */
    const [webSession, setWebSession] = useState<WebSession | null>(null);
    /** web SSE 接続の cleanup 関数 */
    const webCleanupRef = useRef<(() => void) | null>(null);
    /** web モードセッション作成中フラグ */
    const [creating, setCreating] = useState(false);
    /** web モードセッション参加中フラグ */
    const [connecting, setConnecting] = useState(false);
    /** Connect 用に入力された viewer 用コード */
    const [connectCode, setConnectCode] = useState("");
    /** 参加中 DJ 一覧（id → DJ情報） */
    const [roster, setRoster] = useState<Map<string, { djName: string; djImage: string | null }>>(new Map());

    // info アラートを閉じる（スライドアップアニメーション付き）
    const dismissInfo = useCallback(() => {
        setInfoDismissing(true);
        setTimeout(() => {
            setInfoMessage(null);
            setInfoDismissing(false);
        }, 300); // アニメーション duration に合わせる
    }, []);

    // info メッセージが設定されたら自動で閉じる
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

    // 設定から状態を反映するヘルパー
    const applyConfig = (config: AppConfig) => {
        setInfoMessage(`${config.configPath} を読み込みました`);
        setShowComments(config.enableComments);
        setShowTags(config.showTags);
        setEventName(config.eventName);
        setShowEventName(config.showEventName);
        setBackgroundImageConfig(config.backgroundImage);

        if (config.backgroundImage) {
            setBackgroundImagePath(config.backgroundImage.path);
            if (config.backgroundImage.path) {
                // base_dir + path で絶対パスを構築
                const abs = `${config.backgroundImage.baseDir}/${config.backgroundImage.path}`;
                setBackgroundImageAbsolutePath(abs);
            } else {
                setBackgroundImageAbsolutePath(null);
            }
        } else {
            setBackgroundImagePath(null);
            setBackgroundImageAbsolutePath(null);
        }
    };

    /** web セッションに関する表示状態を初期状態に戻す（SSE の切断やサーバーへの通知は行わない） */
    const resetWebSessionView = () => {
        setWebSession(null);
        setSessionCodes(null);
        setTrack(null);
        setRoster(new Map());
    };

    /**
     * web セッションを終了する
     *
     * - SSE を切断する
     * - owner はセッションを破棄し、participant はセッションから退出する（いずれもベストエフォート）
     * - 表示状態を待機画面に戻す
     */
    const endWebSession = (session: WebSession) => {
        // SSE 切断
        if (webCleanupRef.current) {
            webCleanupRef.current();
            webCleanupRef.current = null;
        }
        // サーバー側の後始末
        if (session.role === "owner") {
            destroyWebSession(session);
        } else {
            disconnectWebSession(session);
        }
        platform.clearWebSession().catch(() => { });
        resetWebSessionView();
    };

    // 設定を再読み込みする
    // web モードではセッションを維持する（serverUrl 変更時のみ終了する）
    const handleReloadConfig = async () => {
        setReloading(true);
        setError(null);
        setInfoMessage(null);
        setInfoDismissing(false);
        try {
            const config = await platform.reloadConfig();
            applyConfig(config);

            // web モードで serverUrl が変わった場合はセッションを終了
            if (
                config.mode === "web" &&
                webSession &&
                webSession.serverUrl !== config.web.serverUrl
            ) {
                endWebSession(webSession);
            }

            setAppConfig(config);
        } catch (err) {
            setError(String(err));
        } finally {
            setReloading(false);
        }
    };

    /** セッションを確立した直後の共通処理（Create / Connect） */
    const registerWebSession = (session: WebSession) => {
        setWebSession(session);
        setSessionCodes(session.codes);
        if (session.role === "participant") {
            setConnectCode("");
            // イベント名は、参加したセッションのものを表示する
            if (session.eventName) {
                setEventName(session.eventName);
            }
        }
        // 終了時の破棄 / 退出用にセッション情報を登録する
        platform.setWebSession(session).catch((err) => {
            console.warn("[web] set_web_session failed:", err);
        });
    };

    /** web セッションのコールバック（Create / Connect 共通） */
    const buildWebCallbacks = (): DataSourceCallbacks => ({
        onTrack: (track) => {
            setTrack(track);
            setError(null);
            const djId = track.dirName;
            const djDisplayName = track.djName ?? track.dirName;
            setRoster((prev) => {
                const existing = prev.get(djId);
                if (existing && existing.djName === djDisplayName) return prev;
                const next = new Map(prev);
                next.set(djId, { djName: djDisplayName, djImage: existing?.djImage ?? null });
                return next;
            });
            // モニタウィンドウに転送
            platform.sendTrackToMonitor(track);
        },
        onError: (message) => {
            setError(message);
        },
        onDjJoined: (dj) => {
            setRoster((prev) => {
                const existing = prev.get(dj.id);
                if (existing && existing.djName === dj.djName && existing.djImage === dj.djImage) return prev;
                const next = new Map(prev);
                next.set(dj.id, { djName: dj.djName, djImage: dj.djImage });
                return next;
            });
        },
        onDjLeft: (dj) => {
            setRoster((prev) => {
                if (!prev.has(dj.id)) return prev;
                const next = new Map(prev);
                next.delete(dj.id);
                return next;
            });
        },
        // セッションが破棄された（SSE はすでに閉じている）
        onSessionDestroyed: () => {
            webCleanupRef.current = null;
            platform.clearWebSession().catch(() => { });
            resetWebSessionView();
            setError(null);
            setInfoDismissing(false);
            setInfoMessage("セッションが終了しました");
        },
    });

    // web モード: Create ボタン押下時にセッション作成 + SSE 接続
    const handleCreate = async () => {
        if (!appConfig || appConfig.mode !== "web") return;
        setCreating(true);
        setError(null);

        const cleanup = await createWebSession(appConfig, buildWebCallbacks(), registerWebSession);

        webCleanupRef.current = cleanup;
        setCreating(false);
    };

    // web モード: Connect ボタン押下時に、作成済みセッションへ参加 + SSE 接続
    const handleConnect = async () => {
        if (!appConfig || appConfig.mode !== "web") return;
        if (connectCode.length !== SESSION_CODE_LENGTH) return;
        setConnecting(true);
        setError(null);

        const cleanup = await connectWebSession(
            appConfig.web.serverUrl,
            connectCode,
            buildWebCallbacks(),
            registerWebSession,
        );

        webCleanupRef.current = cleanup;
        setConnecting(false);
    };

    // 背景画像選択ハンドラ
    const handleBackgroundSelect = (entry: BackgroundImageEntry | null) => {
        if (entry === null) {
            setBackgroundImagePath(null);
            setBackgroundImageAbsolutePath(null);
            platform.saveSetting("background_image.path", "null")
                .catch((err) => console.warn("[config writeback] background_image.path:", err));
        } else {
            setBackgroundImagePath(entry.path);
            setBackgroundImageAbsolutePath(entry.absolutePath);
            platform.saveSetting("background_image.path", JSON.stringify(entry.path))
                .catch((err) => console.warn("[config writeback] background_image.path:", err));
        }
        setShowBackgroundPicker(false);
    };

    // trackRef を最新の track に追従させる
    useEffect(() => {
        trackRef.current = track;
    }, [track]);

    // キーボードショートカット（メインウィンドウのみ）
    useEffect(() => {
        if (isMonitor) return;

        const handleKeyDown = (e: KeyboardEvent) => {
            // 入力欄（Connect 用のコード入力など）にフォーカスがあるときはショートカットを無視する
            const target = e.target;
            if (
                target instanceof HTMLElement &&
                (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)
            ) {
                return;
            }

            // イベント名編集中は一切無視
            if (editingEventName) return;

            // BackgroundPicker が開いている場合は一切無視（自前でキー処理する）
            if (showBackgroundPicker) return;

            // ShortcutOverlay が開いている場合は Escape / ? のみ処理
            if (showShortcuts) {
                if (e.key === "Escape" || e.key === "?") {
                    setShowShortcuts(false);
                }
                return;
            }

            switch (e.key) {
                case "r":
                    handleReloadConfig();
                    break;
                case "b":
                    if (backgroundImageConfig) {
                        setShowBackgroundPicker(true);
                    } else {
                        setInfoMessage("背景画像ディレクトリが未設定です");
                    }
                    break;
                case "c":
                    setShowComments((prev) => {
                        const next = !prev;
                        platform.saveSetting("enable_comments", String(next))
                            .catch((err) => console.warn("[config writeback] enable_comments:", err));
                        return next;
                    });
                    break;
                case "t":
                    setShowTags((prev) => {
                        const next = !prev;
                        platform.saveSetting("show_tags", String(next))
                            .catch((err) => console.warn("[config writeback] show_tags:", err));
                        return next;
                    });
                    break;
                case "e":
                    setShowEventName((prev) => {
                        const next = !prev;
                        platform.saveSetting("show_event_name", String(next))
                            .catch((err) => console.warn("[config writeback] show_event_name:", err));
                        return next;
                    });
                    break;
                case "m":
                    platform.openMonitor()
                        .then(() => {
                            // モニタウィンドウが開いた直後に現在のトラック情報を転送
                            if (trackRef.current) {
                                // 少し待ってからイベントリスナー登録後に送る
                                setTimeout(() => {
                                    if (trackRef.current) {
                                        platform.sendTrackToMonitor(trackRef.current);
                                    }
                                }, 100);
                            }
                        })
                        .catch((err) => {
                            console.error("モニタウィンドウの起動に失敗:", err);
                        });
                    break;
                case "S": // Shift+S: セッション破棄（owner）/ 退出（participant）
                    if (webSession) {
                        endWebSession(webSession);
                        setInfoMessage(
                            webSession.role === "owner"
                                ? "セッションを破棄しました"
                                : "セッションから退出しました",
                        );
                    }
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
    }, [isMonitor, backgroundImageConfig, showBackgroundPicker, showShortcuts, editingEventName, webSession]);

    useEffect(() => {
        if (isMonitor) {
            // モニタウィンドウ: メインから転送されたトラック情報を受信
            return platform.listenMonitorTrack((track) => {
                setTrack(track);
            });
        }

        // メインウィンドウ: バックエンドから設定を取得
        let cancelled = false;

        (async () => {
            try {
                const config = await platform.getConfig();

                if (cancelled) return;

                applyConfig(config);
                setAppConfig(config);
            } catch (err) {
                if (!cancelled) {
                    setError(String(err));
                }
            }
        })();

        return () => {
            cancelled = true;
        };
    }, [isMonitor]);

    // local モード用データソースフック（config が取得されたら開始）
    useLocalDataSource(isMonitor ? null : appConfig, {
        onTrack: (track) => {
            setTrack(track);
            setError(null);
            const djId = track.dirName;
            const djDisplayName = track.djName ?? track.dirName;
            setRoster((prev) => {
                const existing = prev.get(djId);
                if (existing && existing.djName === djDisplayName) return prev;
                const next = new Map(prev);
                next.set(djId, { djName: djDisplayName, djImage: existing?.djImage ?? track.djLogoPath });
                return next;
            });
        },
        onError: (message) => {
            setError(message);
        },
        onDjJoined: (dj) => {
            setRoster((prev) => {
                const existing = prev.get(dj.id);
                if (existing && existing.djName === dj.djName && existing.djImage === dj.djImage) return prev;
                const next = new Map(prev);
                next.set(dj.id, { djName: dj.djName, djImage: dj.djImage });
                return next;
            });
        },
        onDjLeft: (dj) => {
            setRoster((prev) => {
                if (!prev.has(dj.id)) return prev;
                const next = new Map(prev);
                next.delete(dj.id);
                return next;
            });
        },
    });

    // モニタウィンドウの場合はコンパクト表示
    if (isMonitor) {
        return <MonitorView track={track} />;
    }

    return (
        <div className="relative flex h-screen flex-col items-center justify-center overflow-hidden bg-black text-white">
            {/* 背景画像レイヤー */}
            {backgroundImageAbsolutePath && (
                <div
                    className="absolute inset-0 z-0"
                    style={{
                        backgroundImage: `url(${platform.convertFileSrc(backgroundImageAbsolutePath)})`,
                        backgroundSize: "cover",
                        backgroundPosition: "center",
                        backgroundRepeat: "no-repeat",
                        opacity: 0.15,
                    }}
                />
            )}
            {/* success アラート（上部固定、スライドダウン/アップ） */}
            {infoMessage && (
                <div
                    className={`absolute left-0 right-0 top-0 z-40 flex items-center justify-between bg-green-900/80 px-4 py-2 text-sm text-green-200 ${infoDismissing ? "animate-slide-up" : "animate-slide-down"
                        }`}
                >
                    <span>{infoMessage}</span>
                    <button
                        onClick={dismissInfo}
                        className="ml-4 cursor-pointer text-green-300 hover:text-white"
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
                        className="ml-4 shrink-0 cursor-pointer rounded bg-red-700 px-3 py-1 text-xs text-red-100 hover:bg-red-600 disabled:opacity-50"
                    >
                        {reloading ? "読込中..." : "再読み込み"}
                    </button>
                </div>
            )}

            <div className="relative z-10 flex h-full w-full flex-col">
                {/* イベント名（track の有無に関わらず表示） */}
                {showEventName && eventName && (
                    <div className="shrink-0 pt-4 text-center">
                        {editingEventName ? (
                            <span className="-mt-1 inline-flex items-center gap-5">
                                <input
                                    type="text"
                                    value={editingEventNameValue}
                                    onChange={(e) => setEditingEventNameValue(e.target.value)}
                                    onKeyDown={(e) => {
                                        if (e.key === "Escape") {
                                            e.stopPropagation();
                                            setEditingEventName(false);
                                        }
                                    }}
                                    autoFocus
                                    className="w-96 rounded border border-gray-600 bg-gray-800 px-2 py-0.5 text-sm text-gray-200 outline-none focus:border-blue-500 md:text-base"
                                />
                                <span className="inline-flex gap-3">
                                    <button
                                        onClick={() => {
                                            const trimmed = editingEventNameValue.trim();
                                            if (trimmed) {
                                                setEventName(trimmed);
                                                platform.saveSetting("event_name", JSON.stringify(trimmed))
                                                    .catch((err) => console.warn("[config writeback] event_name:", err));
                                            }
                                            setEditingEventName(false);
                                        }}
                                        className="cursor-pointer text-green-400 hover:text-green-300"
                                        aria-label="決定"
                                    >
                                        ✓
                                    </button>
                                    <button
                                        onClick={() => setEditingEventName(false)}
                                        className="cursor-pointer text-red-400 hover:text-red-200"
                                        aria-label="キャンセル"
                                    >
                                        ✕
                                    </button>
                                </span>
                            </span>
                        ) : (
                            <span
                                onClick={() => {
                                    setEditingEventNameValue(eventName ?? "");
                                    setEditingEventName(true);
                                }}
                                className="cursor-pointer text-sm text-gray-400 hover:text-gray-200 md:text-base"
                            >
                                {eventName}
                            </span>
                        )}
                    </div>
                )}

                {/* ヘッダ: 参加中 DJ 一覧（track の有無に関わらず表示） */}
                <DjRosterHeader
                    roster={roster}
                    currentDjId={track?.dirName ?? null}
                    djLogoSrc={resolveImageSrc(track?.djLogoPath ?? null, track?.updatedAt ?? "")}
                    sessionCodes={sessionCodes}
                    showSessionEntry={appConfig?.mode === "web" && !webSession}
                    creating={creating}
                    connecting={connecting}
                    connectCode={connectCode}
                    onConnectCodeChange={setConnectCode}
                    onCreate={handleCreate}
                    onConnect={handleConnect}
                />

                <div className={`flex min-h-0 flex-1 w-full flex-col items-center ${track ? "justify-center" : "justify-start"}`}>
                    {track ? (
                        <TrackBody track={track} showComments={showComments} showTags={showTags} />
                    ) : (
                        <WaitingScreen />
                    )}
                </div>
            </div>

            {showShortcuts && <ShortcutOverlay onClose={() => setShowShortcuts(false)} />}

            {showBackgroundPicker && (
                <BackgroundPicker
                    currentPath={backgroundImagePath}
                    onSelect={handleBackgroundSelect}
                    onClose={() => setShowBackgroundPicker(false)}
                />
            )}

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
        { key: "r", description: "設定ファイルの再読み込み" },
        { key: "b", description: "背景画像の選択" },
        { key: "c", description: "コメント表示のトグル" },
        { key: "t", description: "タグ表示のトグル" },
        { key: "e", description: "イベント名表示のトグル" },
        { key: "Shift + s", description: "セッションの破棄 / 退出" },
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
        <div className="pt-48 text-center">
            <h1 className="text-4xl font-bold">now-dj-playing</h1>
            <p className="mt-4 text-lg text-gray-400">
                トラック情報を待機中...
            </p>
        </div>
    );
}

function TrackBody({ track, showComments, showTags }: { track: TrackPayload; showComments: boolean; showTags: boolean }) {
    const artworkSrc = resolveImageSrc(track.artworkPath, track.updatedAt);

    return (
        <main className="flex min-h-0 h-full w-full flex-1 flex-col items-center justify-center gap-8 px-8 pb-8 md:flex-row md:gap-12">
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
    );
}

function DjRosterHeader({
    roster,
    currentDjId,
    djLogoSrc,
    sessionCodes,
    showSessionEntry,
    creating,
    connecting,
    connectCode,
    onConnectCodeChange,
    onCreate,
    onConnect,
}: {
    roster: Map<string, { djName: string; djImage: string | null }>;
    currentDjId: string | null;
    djLogoSrc: string | null;
    sessionCodes: WebSessionCodes | null;
    showSessionEntry: boolean;
    creating: boolean;
    connecting: boolean;
    connectCode: string;
    onConnectCodeChange: (code: string) => void;
    onCreate: () => void;
    onConnect: () => void;
}) {
    const entries = Array.from(roster.entries());
    const busy = creating || connecting;
    const canConnect = !busy && connectCode.length === SESSION_CODE_LENGTH;

    // 未接続のときに表示する Create ボタン + Connect 用のコード入力欄とボタン
    const sessionEntry = showSessionEntry ? (
        <div className="flex items-center gap-3">
            <button
                onClick={onCreate}
                disabled={busy}
                className="cursor-pointer rounded bg-green-600 px-3 py-1 text-base font-semibold text-white hover:bg-green-500 disabled:opacity-50"
            >
                {creating ? "作成中..." : "Create"}
            </button>
            <span className="text-sm text-gray-600">or</span>
            <input
                type="text"
                inputMode="numeric"
                maxLength={SESSION_CODE_LENGTH}
                placeholder="000000"
                value={connectCode}
                onChange={(e) =>
                    onConnectCodeChange(e.target.value.replace(/\D/g, "").slice(0, SESSION_CODE_LENGTH))
                }
                onKeyDown={(e) => {
                    if (e.key === "Enter" && canConnect) {
                        onConnect();
                    }
                }}
                disabled={busy}
                aria-label="viewer 用コード"
                className="w-28 rounded border border-gray-600 bg-gray-800 px-2 py-1 text-center font-mono text-base text-gray-200 outline-none focus:border-blue-500 disabled:opacity-50"
            />
            <button
                onClick={onConnect}
                disabled={!canConnect}
                className="cursor-pointer rounded bg-blue-600 px-3 py-1 text-base font-semibold text-white hover:bg-blue-500 disabled:cursor-default disabled:opacity-50"
            >
                {connecting ? "参加中..." : "Connect"}
            </button>
        </div>
    ) : null;

    // Create した viewer のみ、publisher 用 / viewer 用の認証コードを表示する
    const sessionCodesBadge = !showSessionEntry && sessionCodes ? (
        <span className="flex select-none items-baseline gap-4 text-base text-yellow-300/80">
            <CopyableCode label="publish" code={sessionCodes.publisher} />
            {sessionCodes.viewer && <CopyableCode label="view" code={sessionCodes.viewer} />}
        </span>
    ) : null;

    // n == 0: ロスターが空（Create / Connect の入力 / 認証コードは中央表示）
    if (entries.length === 0) {
        return (
            <header className="relative flex h-[100px] shrink-0 items-center justify-center gap-3 px-8">
                {sessionEntry}
                {sessionCodesBadge}
            </header>
        );
    }

    // n >= 1: 横並び表示 + 現在の DJ をハイライト + 認証コードは右端
    return (
        <header className="relative flex h-[100px] shrink-0 flex-wrap items-center justify-center gap-x-8 gap-y-1 px-8">
            {entries.length === 1 ? (
                <>
                    {(() => {
                        const logoSrc = entries[0]![1].djImage ?? djLogoSrc;
                        if (!logoSrc) return null;
                        const imgSrc = logoSrc.startsWith("data:") ? logoSrc : resolveImageSrc(logoSrc, "");
                        return imgSrc ? (
                            <img
                                src={imgSrc}
                                alt="DJ Logo"
                                className="h-full max-h-[80px] w-auto object-contain"
                            />
                        ) : null;
                    })()}
                    {!(entries[0]![1].djImage ?? djLogoSrc) && (
                        <span className="text-xl font-semibold text-gray-300 md:text-3xl">
                            {entries[0]![1].djName}
                        </span>
                    )}
                </>
            ) : (
                entries.map(([id, info]) => {
                    const isCurrent = id === currentDjId;
                    const logoSrc = info.djImage;
                    return (
                        <span
                            key={id}
                            className={`flex items-center gap-2 px-2 text-xl font-semibold md:text-3xl ${isCurrent
                                ? "border-b-4 border-yellow-500 text-white"
                                : "border-b-4 border-transparent text-gray-500"
                                }`}
                        >
                            {logoSrc ? (
                                <img
                                    src={logoSrc.startsWith("data:") ? logoSrc : (resolveImageSrc(logoSrc, "") ?? "")}
                                    alt="DJ Logo"
                                    className="h-full max-h-[60px] w-auto object-contain"
                                />
                            ) : (
                                info.djName
                            )}
                        </span>
                    );
                })
            )}
            {sessionEntry}
            {sessionCodesBadge && (
                <div className="absolute right-4 top-1/2 -translate-y-1/2">
                    {sessionCodesBadge}
                </div>
            )}
        </header>
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

/**
 * ラベル付きのセッションコード
 *
 * ラベルとコードはマウス選択できない。コードをクリックするとクリップボードにコピーする
 */
function CopyableCode({ label, code }: { label: string; code: string }) {
    const [copied, setCopied] = useState(false);
    const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    // アンマウント時にタイマーを破棄する
    useEffect(() => {
        return () => {
            if (timerRef.current) clearTimeout(timerRef.current);
        };
    }, []);

    const handleCopy = () => {
        navigator.clipboard.writeText(code).then(
            () => {
                // コピーできたことを、一瞬の色の変化で知らせる
                setCopied(true);
                if (timerRef.current) clearTimeout(timerRef.current);
                timerRef.current = setTimeout(() => setCopied(false), 1500);
            },
            (err) => console.warn("[clipboard] コピーに失敗:", err),
        );
    };

    return (
        <span className="select-none">
            <span className="mr-1 text-xs text-gray-500">{label}</span>
            <button
                type="button"
                onClick={handleCopy}
                title="クリックでコピー"
                className={`cursor-pointer select-none font-mono transition-colors ${copied ? "text-green-300" : "hover:text-yellow-200"
                    }`}
            >
                {code}
            </button>
        </span>
    );
}
