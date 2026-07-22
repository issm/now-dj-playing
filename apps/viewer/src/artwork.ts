import { convertFileSrc } from "@tauri-apps/api/core";

/**
 * TrackPayload の artworkPath / djLogoPath から実際に <img src> に渡せる URL を組み立てる
 *
 * - local モード: ローカルファイルパスなので convertFileSrc で変換し、キャッシュバスターを付与する
 * - web モード: Base64 Data URI がそのまま入っているため、変換・キャッシュバスターの付与は行わない
 *   （Data URI はコンテンツ自体が変われば値も変わるため、キャッシュバスターは不要）
 */
export function resolveImageSrc(
  path: string | null,
  cacheBusterKey: string,
): string | null {
  if (!path) return null;

  if (path.startsWith("data:")) {
    return path;
  }

  const cacheBuster = `?t=${encodeURIComponent(cacheBusterKey)}`;
  return convertFileSrc(path) + cacheBuster;
}
