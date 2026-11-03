import { createTauriPlatform } from "./tauri";
import type { Platform } from "./types";

export type { Platform } from "./types";

/**
 * 現在の実行環境に合った Platform
 *
 * 現時点では Tauri 版のみ。ブラウザ対応（#77）で、実行時判定による切り替えを追加する。
 */
export const platform: Platform = createTauriPlatform();
