use std::fs;
use std::path::Path;

use crate::types::{DjProfile, DjState, NowPlaying, ReadyManifest};

/// .ready ファイルを解析する
pub fn parse_ready(path: &Path) -> Result<ReadyManifest, ParseError> {
    let content = fs::read_to_string(path).map_err(|e| ParseError::Io(path.to_path_buf(), e))?;
    let manifest: ReadyManifest =
        serde_json::from_str(&content).map_err(|e| ParseError::Json(path.to_path_buf(), e))?;
    Ok(manifest)
}

/// now_playing.json を解析する
pub fn parse_now_playing(path: &Path) -> Result<NowPlaying, ParseError> {
    let content = fs::read_to_string(path).map_err(|e| ParseError::Io(path.to_path_buf(), e))?;
    let np: NowPlaying =
        serde_json::from_str(&content).map_err(|e| ParseError::Json(path.to_path_buf(), e))?;
    Ok(np)
}

/// DJ プロファイルを解決する
///
/// 優先順位:
/// 1. dj-profile (拡張子なし) or dj-profile.txt → テキスト内容を DJ 名として使用
/// 2. dj-profile.png / .jpg / .jpeg → ロゴ画像
/// 3. どれも存在しない → ディレクトリ名をフォールバック
pub fn resolve_dj_profile(dj_dir: &Path) -> DjProfile {
    // テキスト系を優先
    for filename in &["dj-profile", "dj-profile.txt"] {
        let path = dj_dir.join(filename);
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                let name = content.trim().to_string();
                if !name.is_empty() {
                    return DjProfile::Name(name);
                }
            }
        }
    }

    // 画像系
    for ext in &["png", "jpg", "jpeg"] {
        let path = dj_dir.join(format!("dj-profile.{}", ext));
        if path.is_file() {
            return DjProfile::Logo(path);
        }
    }

    // フォールバック: ディレクトリ名
    let dir_name = dj_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown DJ")
        .to_string();
    DjProfile::Name(dir_name)
}

/// .ready マニフェストに基づいて DJ の状態を構築する
pub fn build_dj_state(dj_dir: &Path, manifest: &ReadyManifest) -> Result<DjState, ParseError> {
    // now_playing.json は必須
    if !manifest.files.contains(&"now_playing.json".to_string()) {
        return Err(ParseError::MissingFile(
            dj_dir.to_path_buf(),
            "now_playing.json".to_string(),
        ));
    }

    let now_playing = parse_now_playing(&dj_dir.join("now_playing.json"))?;
    let profile = resolve_dj_profile(dj_dir);

    // artwork はマニフェストに含まれるもののみ使用
    let artwork_path = manifest
        .files
        .iter()
        .find(|f| {
            let f_lower = f.to_lowercase();
            f_lower.starts_with("artwork.")
                && (f_lower.ends_with(".png")
                    || f_lower.ends_with(".jpg")
                    || f_lower.ends_with(".jpeg"))
        })
        .map(|f| dj_dir.join(f))
        .filter(|p| p.is_file());

    let dir_name = dj_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(DjState {
        dir_name,
        profile,
        now_playing,
        artwork_path,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("IO error reading {0}: {1}")]
    Io(std::path::PathBuf, std::io::Error),
    #[error("JSON parse error in {0}: {1}")]
    Json(std::path::PathBuf, serde_json::Error),
    #[error("Missing required file {1} in {0}")]
    MissingFile(std::path::PathBuf, String),
}

/// ベースディレクトリ内の既存 .ready を走査し、有効な DjState を返す
pub fn scan_existing(base_dir: &Path) -> Vec<DjState> {
    let mut results = Vec::new();

    let entries = match fs::read_dir(base_dir) {
        Ok(entries) => entries,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let ready_path = path.join(".ready");
        if !ready_path.is_file() {
            continue;
        }

        match parse_ready(&ready_path) {
            Ok(manifest) => match build_dj_state(&path, &manifest) {
                Ok(state) => results.push(state),
                Err(e) => {
                    log::warn!("スキャン中にエラー: {}", e);
                }
            },
            Err(e) => {
                log::warn!("スキャン中にエラー: {}", e);
            }
        }
    }

    results
}
