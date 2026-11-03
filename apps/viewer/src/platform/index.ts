import { isTauri } from "@tauri-apps/api/core";
import { createBrowserPlatform } from "./browser";
import { createTauriPlatform } from "./tauri";
import type { Platform } from "./types";

export type { Platform, PlatformCapabilities } from "./types";

/**
 * 現在の実行環境に合った Platform
 *
 * 実行時に判定する。`vite dev` で開いたブラウザでも、同じコードが動く。
 * Tauri の外で `createTauriPlatform()` を呼ぶと例外になるため、Tauri のときだけ生成する。
 */
export const platform: Platform = isTauri()
  ? createTauriPlatform()
  : createBrowserPlatform();
