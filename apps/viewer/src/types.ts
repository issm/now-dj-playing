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

/** Rust 側から返されるアプリ設定 */
export interface AppConfig {
  watchDir: string;
  djId: string;
  enableComments: boolean;
  showTags: boolean;
  /** イベント名（省略時は null） */
  eventName: string | null;
  /** イベント名を表示するかどうか（デフォルト: true） */
  showEventName: boolean;
  configPath: string;
}

/** Rust 側から返されるバージョン情報 */
export interface VersionInfo {
  /** SemVer バージョン (例: "0.1.0") */
  version: string;
  /** ビルドメタデータ (例: "20260704T123045.a1b2c3d") */
  buildMetadata: string;
  /** ビルド時刻 (例: "20260704T123045") */
  buildTimestamp: string;
  /** git commit hash (例: "a1b2c3d") */
  commitHash: string;
  /** フル表記 (例: "0.1.0+20260704T123045.a1b2c3d") */
  full: string;
}
