/** Rust 側から emit される楽曲情報 */
export interface TrackPayload {
  dirName: string;
  djName: string | null;
  djLogoPath: string | null;
  title: string;
  artist: string;
  album: string | null;
  comment: string | null;
  artworkPath: string | null;
  updatedAt: string;
}

/** Rust 側から emit されるエラー情報 */
export interface ErrorPayload {
  dirName: string;
  message: string;
}
