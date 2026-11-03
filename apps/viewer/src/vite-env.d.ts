/// <reference types="vite/client" />

// vite.config.ts の define で注入されるグローバル定数
declare const __APP_VERSION__: string;
declare const __BUILD_METADATA__: string;
declare const __BUILD_TIMESTAMP__: string;
declare const __BUILD_COMMIT_HASH__: string;
declare const __APP_FULL_VERSION__: string;

// ブラウザ版のビルド時の環境変数（.env.web）
interface ImportMetaEnv {
  /** ndp-server のベース URL（末尾のスラッシュなし。空文字は同一オリジン） */
  readonly VITE_NDP_SERVER_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
