import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";

type Mode = "web" | "local";
type PublishStatus = "idle" | "success" | "error";

interface Config {
  dj_name: string;
  local: { dj_id: string; publish_base_dir: string };
  web: { endpoint_url: string };
}

interface PublishResult {
  title: string;
  artist: string;
  artwork: string | null;
}

/** 入力値の OK/NG に応じたボーダー色を返す */
function inputBorder(valid: boolean): string {
  return valid ? "border-green-600" : "border-red-600";
}

function App() {
  const [mode, setMode] = useState<Mode>("web");
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

  // web モード
  const [endpointUrl, setEndpointUrl] = useState("");
  const [code, setCode] = useState("");

  // local モード
  const [djId, setDjId] = useState("dj-000");
  const [publishBaseDir, setPublishBaseDir] = useState("");
  const [publishBaseDirExists, setPublishBaseDirExists] = useState(false);

  // 起動時に config を読み込み
  useEffect(() => {
    invoke<Config>("load_config")
      .then((config) => {
        if (config.dj_name) setDjName(config.dj_name);
        if (config.local?.dj_id) setDjId(config.local.dj_id);
        if (config.local?.publish_base_dir)
          setPublishBaseDir(config.local.publish_base_dir);
        if (config.web?.endpoint_url) setEndpointUrl(config.web.endpoint_url);
      })
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

  // Tauri ドロップイベントの登録
  const doPublish = useCallback(
    async (filePath: string) => {
      setPublishStatus("idle");
      setPublishError("");
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
    [mode, djName, endpointUrl, code, djId, publishBaseDir],
  );

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setIsDragOver(true);
      } else if (event.payload.type === "drop") {
        setIsDragOver(false);
        const paths = event.payload.paths;
        if (paths.length > 0) {
          doPublish(paths[0]);
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
    // TODO: leave API が実装されたら呼び出す
    setJoined(false);
  };

  const handleReloadConfig = async () => {
    try {
      const config = await invoke<Config>("load_config");
      if (config.dj_name) setDjName(config.dj_name);
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

  // web モードで join 済みの場合、入力を無効化
  const webInputDisabled = mode === "web" && joined;

  return (
    <div className="flex flex-col h-screen p-3 gap-2 text-sm">
      {/* モードタブ + メニュー */}
      <div className="flex gap-1">
        <button
          className={`flex-1 py-1 rounded text-xs font-bold ${mode === "web" ? "bg-blue-600" : "bg-gray-700"}`}
          onClick={() => setMode("web")}
        >
          web
        </button>
        <button
          className={`flex-1 py-1 rounded text-xs font-bold ${mode === "local" ? "bg-blue-600" : "bg-gray-700"}`}
          onClick={() => setMode("local")}
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

      {/* DJ 名 */}
      <div className="flex items-center gap-2">
        <label className="shrink-0 text-xs text-gray-400">DJ 名</label>
        <input
          className={`flex-1 bg-gray-800 border rounded px-2 py-1 text-xs ${inputBorder(djName.length > 0)} ${webInputDisabled ? "opacity-50" : ""}`}
          value={djName}
          onChange={(e) => setDjName(e.target.value)}
          disabled={webInputDisabled}
        />
      </div>

      {/* モード設定 */}
      {mode === "web" ? (
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
      ) : (
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

      {/* ドロップ領域 */}
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
          <div className="relative z-10 flex flex-col items-center justify-end h-full pb-3 px-2">
            <p className="text-white text-xs font-bold text-center truncate w-full">
              {lastTrack.title}
            </p>
            <p className="text-gray-300 text-[10px] text-center truncate w-full">
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
    </div>
  );
}

export default App;
