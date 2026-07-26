import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";

type Tab = "base" | "web" | "local";
type PublishStatus = "idle" | "success" | "error";

interface Config {
  dj_name: string;
  dj_image: string | null;
  local: { dj_id: string; publish_base_dir: string };
  web: { endpoint_url: string };
}

interface PublishResult {
  title: string;
  artist: string;
  artwork: string | null;
}

/** 画像ファイル拡張子の判定 */
function isImageFile(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"].includes(ext);
}

/** 入力値の OK/NG に応じたボーダー色を返す */
function inputBorder(valid: boolean): string {
  return valid ? "border-green-600" : "border-red-600";
}

function App() {
  const [tab, setTab] = useState<Tab>("base");
  const [isDragOver, setIsDragOver] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);

  // publish 結果 (ドロップ領域に連動)
  const [publishStatus, setPublishStatus] = useState<PublishStatus>("idle");
  const [publishError, setPublishError] = useState("");
  const [lastTrack, setLastTrack] = useState<PublishResult | null>(null);

  // join 結果 (認証コードに連動)
  const [joined, setJoined] = useState(false);
  const [joinFailed, setJoinFailed] = useState(false);

  // 共通
  const [djName, setDjName] = useState("");
  const [djImage, setDjImage] = useState<string | null>(null);
  const [djImageDataUri, setDjImageDataUri] = useState<string | null>(null);

  // web モード
  const [endpointUrl, setEndpointUrl] = useState("");
  const [code, setCode] = useState("");

  // local モード
  const [djId, setDjId] = useState("dj-000");
  const [publishBaseDir, setPublishBaseDir] = useState("");
  const [publishBaseDirExists, setPublishBaseDirExists] = useState(false);

  // バージョン情報
  const [versionInfo, setVersionInfo] = useState<{ gui: string } | null>(null);

  // タブの最新値を ref で保持（ドロップイベントコールバックで参照するため）
  const tabRef = useRef<Tab>(tab);
  useEffect(() => {
    tabRef.current = tab;
  }, [tab]);

  // 起動時に config とバージョンを読み込み
  useEffect(() => {
    invoke<Config>("load_config")
      .then((config) => {
        if (config.dj_name) setDjName(config.dj_name);
        if (config.dj_image) setDjImage(config.dj_image);
        if (config.local?.dj_id) setDjId(config.local.dj_id);
        if (config.local?.publish_base_dir)
          setPublishBaseDir(config.local.publish_base_dir);
        if (config.web?.endpoint_url) setEndpointUrl(config.web.endpoint_url);
      })
      .catch(() => { });

    invoke<{ gui: string }>("get_version")
      .then(setVersionInfo)
      .catch(() => { });
  }, []);

  // 出力先ディレクトリの存在確認
  useEffect(() => {
    if (publishBaseDir) {
      invoke<boolean>("check_dir_exists", { path: publishBaseDir }).then(
        setPublishBaseDirExists,
      );
    } else {
      setPublishBaseDirExists(false);
    }
  }, [publishBaseDir]);

  // dj_image パスから Data URI を取得
  useEffect(() => {
    if (djImage) {
      invoke<string>("read_image_as_data_uri", { path: djImage })
        .then(setDjImageDataUri)
        .catch(() => setDjImageDataUri(null));
    } else {
      setDjImageDataUri(null);
    }
  }, [djImage]);

  // publish 実行
  const doPublish = useCallback(
    async (filePath: string) => {
      setPublishStatus("idle");
      setPublishError("");
      const mode = tabRef.current === "base" ? "web" : tabRef.current;
      try {
        const result = await invoke<PublishResult>("publish", {
          filePath,
          mode,
          djName,
          endpointUrl,
          code: code || null,
          djId,
          publishBaseDir,
        });
        setPublishStatus("success");
        setLastTrack(result);
      } catch (err) {
        setPublishStatus("error");
        setPublishError(String(err));
      }
    },
    [djName, endpointUrl, code, djId, publishBaseDir],
  );

  // Tauri ドロップイベント: タブに応じて振り分け
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setIsDragOver(true);
      } else if (event.payload.type === "drop") {
        setIsDragOver(false);
        const paths = event.payload.paths;
        if (paths.length > 0) {
          const droppedPath = paths[0];
          if (tabRef.current === "base") {
            // 基本タブ: 画像ファイルなら dj_image にセット
            if (isImageFile(droppedPath)) {
              setDjImage(droppedPath);
            }
          } else {
            // web/local タブ: publish
            doPublish(droppedPath);
          }
        }
      } else {
        setIsDragOver(false);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [doPublish]);

  const handleJoin = async () => {
    setJoinFailed(false);
    try {
      await invoke("join_session", { endpointUrl, code, djName });
      setJoined(true);
    } catch (_) {
      setJoinFailed(true);
    }
  };

  const handleLeave = async () => {
    try {
      await invoke("leave_session");
    } catch (_) {
      // leave 失敗時も UI 上は離脱扱いにする（セッション無効の可能性）
    }
    setJoined(false);
    setCode("");
    setPublishStatus("idle");
    setPublishError("");
    setLastTrack(null);
  };

  const handleReloadConfig = async () => {
    try {
      const config = await invoke<Config>("load_config");
      if (config.dj_name) setDjName(config.dj_name);
      setDjImage(config.dj_image ?? null);
      if (config.local?.dj_id) setDjId(config.local.dj_id);
      if (config.local?.publish_base_dir)
        setPublishBaseDir(config.local.publish_base_dir);
      if (config.web?.endpoint_url) setEndpointUrl(config.web.endpoint_url);
    } catch { }
  };

  const handleSaveConfig = async () => {
    try {
      await invoke("save_config", {
        djName,
        djImage: djImage || null,
        djId,
        publishBaseDir,
        endpointUrl,
      });
    } catch { }
  };

  const handleOpenFolder = async () => {
    try {
      await invoke("open_config_folder");
    } catch { }
  };

  const handleClearDjImage = () => {
    setDjImage(null);
    setDjImageDataUri(null);
  };

  // web モードで join 済みの場合、入力を無効化
  const webInputDisabled = tab === "web" && joined;

  return (
    <div className="flex flex-col h-screen p-3 gap-2 text-sm">
      {/* タブ + メニュー */}
      <div className="flex gap-1">
        <button
          className={`flex-1 py-1 rounded text-xs font-bold ${tab === "base" ? "bg-blue-600" : "bg-gray-700"}`}
          onClick={() => setTab("base")}
        >
          基本
        </button>
        <button
          className={`flex-1 py-1 rounded text-xs font-bold ${tab === "web" ? "bg-purple-600" : "bg-gray-700"}`}
          onClick={() => setTab("web")}
        >
          web
        </button>
        <button
          className={`flex-1 py-1 rounded text-xs font-bold ${tab === "local" ? "bg-purple-600" : "bg-gray-700"}`}
          onClick={() => setTab("local")}
        >
          local
        </button>
        <div className="relative">
          <button
            className="px-2 py-1 rounded text-xs bg-gray-700 hover:bg-gray-600"
            onClick={() => setMenuOpen(!menuOpen)}
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
            </svg>
          </button>
          {menuOpen && (
            <div className="absolute right-0 top-full mt-1 bg-gray-800 border border-gray-600 rounded shadow-lg z-50 min-w-[140px]">
              <button
                className="w-full text-left px-3 py-2 text-xs hover:bg-gray-700"
                onClick={() => { handleReloadConfig(); setMenuOpen(false); }}
              >
                Config リロード
              </button>
              <button
                className="w-full text-left px-3 py-2 text-xs hover:bg-gray-700"
                onClick={() => { handleSaveConfig(); setMenuOpen(false); }}
              >
                Config 保存
              </button>
              <button
                className="w-full text-left px-3 py-2 text-xs hover:bg-gray-700"
                onClick={() => { handleOpenFolder(); setMenuOpen(false); }}
              >
                フォルダを開く
              </button>
            </div>
          )}
        </div>
      </div>

      {/* タブコンテンツ */}
      {tab === "base" && (
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <label className="text-xs text-gray-400">DJ 名</label>
            {djImage && (
              <button
                className="text-[10px] text-red-400 hover:text-red-300"
                onClick={handleClearDjImage}
              >
                画像クリア
              </button>
            )}
          </div>
          <input
            className={`w-full bg-gray-800 border rounded px-2 py-1 text-xs ${inputBorder(djName.length > 0)}`}
            value={djName}
            onChange={(e) => setDjName(e.target.value)}
          />
          <div
            className={`flex items-center justify-center border-2 border-dashed rounded-lg h-24 transition-colors ${isDragOver && tab === "base"
              ? "border-blue-400 bg-blue-900/30"
              : djImage
                ? "border-green-600 bg-gray-800/50"
                : "border-gray-600 bg-gray-800/50"
              }`}
          >
            {djImage ? (
              <img
                src={djImageDataUri ?? ""}
                alt="DJ 画像"
                className="max-h-20 max-w-full object-contain rounded"
              />
            ) : (
              <span className="text-gray-500 text-xs">
                画像をドロップ
              </span>
            )}
          </div>
          {djImage && (
            <p className="text-[10px] text-gray-500 truncate" title={djImage}>
              {djImage}
            </p>
          )}
        </div>
      )}

      {tab === "web" && (
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <label className="shrink-0 text-xs text-gray-400">エンドポイント</label>
            <input
              className={`flex-1 bg-gray-800 border rounded px-2 py-1 text-xs ${inputBorder(endpointUrl.length > 0)} ${webInputDisabled ? "opacity-50" : ""}`}
              placeholder="http://localhost:8080/api"
              value={endpointUrl}
              onChange={(e) => setEndpointUrl(e.target.value)}
              disabled={webInputDisabled}
            />
          </div>
          <div className="flex items-center gap-2">
            <label className="shrink-0 text-xs text-gray-400">認証コード</label>
            <input
              className={`flex-1 bg-gray-800 border rounded px-2 py-1 text-xs ${joined ? "border-green-600 opacity-50" : joinFailed ? "border-red-600" : "border-gray-600"}`}
              placeholder="000000"
              maxLength={6}
              value={code}
              onChange={(e) => { setCode(e.target.value); setJoinFailed(false); }}
              disabled={webInputDisabled}
            />
            {joined ? (
              <button
                className="bg-red-700 hover:bg-red-600 px-2 py-1 rounded text-xs"
                onClick={handleLeave}
              >
                Leave
              </button>
            ) : (
              <button
                className={`px-2 py-1 rounded text-xs ${code.length === 6 ? "bg-green-700 hover:bg-green-600" : "bg-gray-600 opacity-50 cursor-not-allowed"}`}
                onClick={handleJoin}
                disabled={code.length !== 6}
              >
                Join
              </button>
            )}
          </div>
        </div>
      )}

      {tab === "local" && (
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <label className="shrink-0 text-xs text-gray-400">DJ ID</label>
            <input
              className={`flex-1 bg-gray-800 border rounded px-2 py-1 text-xs ${inputBorder(djId.length > 0)}`}
              value={djId}
              onChange={(e) => setDjId(e.target.value)}
            />
          </div>
          <div className="flex items-center gap-2">
            <label className="shrink-0 text-xs text-gray-400">出力先</label>
            <input
              className={`flex-1 bg-gray-800 border rounded px-2 py-1 text-xs ${inputBorder(publishBaseDirExists)}`}
              placeholder="~/tmp/ndp"
              value={publishBaseDir}
              onChange={(e) => setPublishBaseDir(e.target.value)}
            />
            <button
              className="bg-gray-600 hover:bg-gray-500 px-2 py-1 rounded text-xs"
              onClick={async () => {
                const selected = await open({ directory: true });
                if (selected) setPublishBaseDir(selected as string);
              }}
            >
              ...
            </button>
          </div>
        </div>
      )}

      {/* ドロップ領域 (web/local タブのみ) */}
      {tab !== "base" && (
        <>
          <div
            className={`relative flex-1 flex flex-col items-center justify-center border-2 border-dashed rounded-lg transition-colors overflow-hidden ${isDragOver
              ? "border-blue-400 bg-blue-900/30"
              : publishStatus === "success"
                ? "border-green-500 bg-gray-800/50"
                : publishStatus === "error"
                  ? "border-red-500 bg-gray-800/50"
                  : "border-gray-600 bg-gray-800/50"
              }`}
          >
            {/* アートワーク背景 */}
            {lastTrack?.artwork && (
              <div
                className="absolute inset-0 bg-cover bg-center opacity-30"
                style={{ backgroundImage: `url(${lastTrack.artwork})` }}
              />
            )}

            {/* トラック情報 or プレースホルダー */}
            {lastTrack ? (
              <div className="relative z-10 flex flex-col gap-1 justify-end h-full pb-3 px-2 w-full">
                <p className="text-white text-sm font-bold text-left break-words">
                  {lastTrack.title}
                </p>
                <p className="text-gray-300 text-xs text-left break-words">
                  {lastTrack.artist}
                </p>
              </div>
            ) : (
              <span className="text-gray-500 text-xs relative z-10">
                ここにファイルをドロップ
              </span>
            )}
          </div>

          {/* publish エラー時のみメッセージ表示 */}
          {publishStatus === "error" && publishError && (
            <p className="text-red-300 text-[10px] truncate max-w-full text-center">
              {publishError}
            </p>
          )}
        </>
      )}

      {/* バージョン情報 (常にウィンドウ下部) */}
      <div className="mt-auto">
        {versionInfo && (
          <div className="text-[9px] text-gray-500 text-right leading-tight">
            <div>Version: {versionInfo.gui}</div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
