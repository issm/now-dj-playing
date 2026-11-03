import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { execSync } from "child_process";
import { readFileSync } from "fs";
import { resolve } from "path";

// tauri.conf.json からバージョンを取得（single source of truth）
const tauriConf = JSON.parse(
  readFileSync(resolve(__dirname, "src-tauri/tauri.conf.json"), "utf-8"),
);
const version: string = tauriConf.version;

// 対象コミットのタイムスタンプ (UTC) を yyyymmddThhmmss 形式で取得
let buildTimestamp = "00000000T000000";
try {
  buildTimestamp = execSync(
    'TZ=UTC git log -1 --format=%cd --date=format:%Y%m%dT%H%M%S',
    { encoding: "utf-8" },
  ).trim();
} catch {
  // git が利用できない環境ではフォールバック
}

// git commit hash (short)
let commitHash = "unknown";
try {
  commitHash = execSync("git rev-parse --short HEAD", { encoding: "utf-8" }).trim();
} catch {
  // git が利用できない環境ではフォールバック
}

const buildMetadata = `${buildTimestamp}.${commitHash}`;

// 開発時は "-dev" サフィックスを付与
const devSuffix = process.env.NODE_ENV === "production" ? "" : "-dev";
const fullVersion = `${version}+${buildMetadata}${devSuffix}`;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig(({ mode }) => {
  // ブラウザ版（--mode web）: サーバー URL をビルド時の環境変数で固定する
  const isWeb = mode === "web";
  if (isWeb) {
    const env = loadEnv(mode, __dirname, "VITE_");
    // 空文字は有効（同一オリジンで配信する場合の相対パス）。未設定のみ失敗させる
    if (env.VITE_NDP_SERVER_URL === undefined) {
      throw new Error(
        "VITE_NDP_SERVER_URL が未設定です。" +
        ".env.web に記述する（.env.web.example を参照）か、環境変数で指定してください。",
      );
    }
  }

  return {
    plugins: [react(), tailwindcss()],
    clearScreen: false,
    define: {
      __APP_VERSION__: JSON.stringify(version),
      __BUILD_METADATA__: JSON.stringify(buildMetadata),
      __BUILD_TIMESTAMP__: JSON.stringify(buildTimestamp),
      __BUILD_COMMIT_HASH__: JSON.stringify(commitHash),
      __APP_FULL_VERSION__: JSON.stringify(fullVersion),
    },
    build: {
      // ブラウザ版は dist-web に出力する（Tauri 用の dist とは分ける）
      outDir: isWeb ? "dist-web" : "dist",
    },
    server: {
      port: 1420,
      strictPort: true,
      watch: {
        ignored: ["**/src-tauri/**"],
      },
    },
  };
});
