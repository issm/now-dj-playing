import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";

type Mode = "web" | "local";
type Status = "idle" | "success" | "error";

interface Config {
  dj_name: string;
  local: { dj_id: string; publish_base_dir: string };
  web: { endpoint_url: string };
}

function App() {
  const [mode, setMode] = useState<Mode>("web");
  const [status, setStatus] = useState<Status>("idle");
  const [errorMsg, setErrorMsg] = useState("");
  const [isDragOver, setIsDragOver] = useState(false);

  // 共通
  const [djName, setDjName] = useState("");

  // web モード
  const [endpointUrl, setEndpointUrl] = useState("");
  const [code, setCode] = useState("");

  // local モード
  const [djId, setDjId] = useState("dj-000");
  const [publishBaseDir, setPublishBaseDir] = useState("");

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

  // Tauri ドロップイベントの登録
  const doPublish = useCallback(
    async (filePath: string) => {
      setStatus("idle");
      setErrorMsg("");
      try {
        await invoke("publish", {
          filePath,
          mode,
          djName,
          endpointUrl,
          code: code || null,
          djId,
          publishBaseDir,
        });
        setStatus("success");
      } catch (err) {
        setStatus("error");
        setErrorMsg(String(err));
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
        // cancel
        setIsDragOver(false);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [doPublish]);

  const handleJoin = async () => {
    setStatus("idle");
    setErrorMsg("");
    try {
      await invoke("join_session", { endpointUrl, code, djName });
      setStatus("success");
    } catch (err) {
      setStatus("error");
      setErrorMsg(String(err));
    }
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

  return (
    <div className="flex flex-col h-screen p-3 gap-2 text-sm">
      {/* DJ 名 */}
      <div className="flex items-center gap-2">
        <label className="shrink-0 text-xs text-gray-400">DJ 名</label>
        <input
          className="flex-1 bg-gray-800 border border-gray-600 rounded px-2 py-1 text-xs"
          value={djName}
          onChange={(e) => setDjName(e.target.value)}
        />
      </div>

      {/* モードタブ */}
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
      </div>

      {/* モード設定 */}
      {mode === "web" ? (
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <label className="shrink-0 text-xs text-gray-400">EP</label>
            <input
              className="flex-1 bg-gray-800 border border-gray-600 rounded px-2 py-1 text-xs"
              placeholder="http://localhost:8080/api"
              value={endpointUrl}
              onChange={(e) => setEndpointUrl(e.target.value)}
            />
          </div>
          <div className="flex items-center gap-2">
            <label className="shrink-0 text-xs text-gray-400">Code</label>
            <input
              className="flex-1 bg-gray-800 border border-gray-600 rounded px-2 py-1 text-xs"
              placeholder="000000"
              maxLength={6}
              value={code}
              onChange={(e) => setCode(e.target.value)}
            />
            <button
              className="bg-green-700 hover:bg-green-600 px-2 py-1 rounded text-xs"
              onClick={handleJoin}
            >
              Join
            </button>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <label className="shrink-0 text-xs text-gray-400">DJ ID</label>
            <input
              className="flex-1 bg-gray-800 border border-gray-600 rounded px-2 py-1 text-xs"
              value={djId}
              onChange={(e) => setDjId(e.target.value)}
            />
          </div>
          <div className="flex items-center gap-2">
            <label className="shrink-0 text-xs text-gray-400">出力先</label>
            <input
              className="flex-1 bg-gray-800 border border-gray-600 rounded px-2 py-1 text-xs"
              placeholder="~/tmp/ndp"
              value={publishBaseDir}
              onChange={(e) => setPublishBaseDir(e.target.value)}
            />
          </div>
        </div>
      )}

      {/* ドロップ領域 */}
      <div
        className={`flex-1 flex items-center justify-center border-2 border-dashed rounded-lg transition-colors ${isDragOver
            ? "border-blue-400 bg-blue-900/30"
            : "border-gray-600 bg-gray-800/50"
          }`}
      >
        <span className="text-gray-500 text-xs">ここにファイルをドロップ</span>
      </div>

      {/* ステータス */}
      <div className="min-h-[24px] flex items-center justify-center">
        {status === "success" && (
          <span className="text-green-400 text-xs font-bold">● 成功</span>
        )}
        {status === "error" && (
          <div className="text-center">
            <span className="text-red-400 text-xs font-bold">● 失敗</span>
            {errorMsg && (
              <p className="text-red-300 text-[10px] mt-0.5 truncate max-w-full">
                {errorMsg}
              </p>
            )}
          </div>
        )}
      </div>

      {/* Config ボタン */}
      <div className="flex gap-2">
        <button
          className="flex-1 bg-gray-700 hover:bg-gray-600 py-1 rounded text-xs"
          onClick={handleReloadConfig}
        >
          Config リロード
        </button>
        <button
          className="flex-1 bg-gray-700 hover:bg-gray-600 py-1 rounded text-xs"
          onClick={handleSaveConfig}
        >
          Config 保存
        </button>
      </div>
    </div>
  );
}

export default App;
