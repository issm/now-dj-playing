/// <reference types="vite/client" />

// vite.config.ts の define で注入されるグローバル定数
declare const __APP_VERSION__: string;
declare const __BUILD_METADATA__: string;
declare const __BUILD_TIMESTAMP__: string;
declare const __BUILD_COMMIT_HASH__: string;
declare const __APP_FULL_VERSION__: string;
