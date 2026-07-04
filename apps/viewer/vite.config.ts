import { defineConfig } from "vite";
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
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(version),
    __BUILD_METADATA__: JSON.stringify(buildMetadata),
    __BUILD_TIMESTAMP__: JSON.stringify(buildTimestamp),
    __BUILD_COMMIT_HASH__: JSON.stringify(commitHash),
    __APP_FULL_VERSION__: JSON.stringify(fullVersion),
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
