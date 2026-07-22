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

/** 背景画像設定 */
export interface BackgroundImageConfig {
  /** 背景画像ディレクトリの絶対パス */
  baseDir: string;
  /** base_dir からの相対パス（null で「なし」） */
  path: string | null;
}

/** 背景画像一覧のエントリ */
export interface BackgroundImageEntry {
  /** base_dir からの相対パス（ファイル名） */
  path: string;
  /** 絶対パス（convertFileSrc 用） */
  absolutePath: string;
}

/** データソースモード */
export type Mode = "local" | "web";

/** local モード固有の設定 */
export interface LocalConfig {
  watchDir: string;
  djId: string;
}

/** web モード固有の設定 */
export interface WebConfig {
  serverUrl: string;
}

/** Rust 側から返されるアプリ設定 */
export interface AppConfig {
  /** データソースモード */
  mode: Mode;
  /** local モード固有の設定 */
  local: LocalConfig;
  /** web モード固有の設定 */
  web: WebConfig;
  enableComments: boolean;
  showTags: boolean;
  /** イベント名（省略時は null） */
  eventName: string | null;
  /** イベント名を表示するかどうか（デフォルト: true） */
  showEventName: boolean;
  /** 背景画像設定（省略時は null = 機能無効） */
  backgroundImage: BackgroundImageConfig | null;
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
