use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use std::path::PathBuf;

/// .ready マニフェストファイルの構造
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyManifest {
    pub updated_at: DateTime<FixedOffset>,
    pub files: Vec<String>,
}

/// now_playing.json の構造
#[derive(Debug, Clone, Deserialize)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub artwork: Option<String>,
    pub updated_at: DateTime<FixedOffset>,
}

/// DJ プロファイルの種別
#[derive(Debug, Clone)]
pub enum DjProfile {
    /// テキスト名
    Name(String),
    /// ロゴ画像ファイルパス
    Logo(PathBuf),
}

/// DJ 1人分の再生情報（解析済み）
#[derive(Debug, Clone)]
pub struct DjState {
    /// DJ ディレクトリ名
    pub dir_name: String,
    /// DJ プロファイル
    pub profile: DjProfile,
    /// 現在の再生情報
    pub now_playing: NowPlaying,
    /// アートワーク画像のパス (あれば)
    pub artwork_path: Option<PathBuf>,
}

/// Watcher から発行されるイベント
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// DJ の再生情報が更新された
    TrackChanged(DjState),
    /// DJ ディレクトリが削除された等
    DjRemoved { dir_name: String },
    /// パースエラー等
    Error { dir_name: String, message: String },
}
