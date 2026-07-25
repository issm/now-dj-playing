//! local モード: ファイルシステムへの書き出し

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;

use crate::tags::TrackMeta;

#[derive(Serialize)]
struct NowPlaying {
    title: String,
    artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artwork: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    updated_at: String,
}

#[derive(Serialize)]
struct ReadyManifest {
    updated_at: String,
    files: Vec<String>,
}

/// local モードで楽曲情報をファイルに書き出す
pub fn publish_local(
    meta: &TrackMeta,
    out: &Path,
    id: &str,
    dj_name: Option<&str>,
) -> Result<()> {
    // 出力先ディレクトリの決定: {out}/{id}/
    let out_dir = out.join(id);
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("出力先ディレクトリの作成に失敗: {}", out_dir.display()))?;

    // DJ プロファイルの書き出し
    if let Some(name) = dj_name {
        write_dj_profile(&out_dir, name)?;
    }

    // アートワーク抽出
    let mut artwork_filename: Option<String> = None;
    let mut files = vec!["now_playing.json".to_string()];

    if let Some(artwork) = &meta.artwork {
        let img_ext = if artwork.mime.contains("png") {
            "png"
        } else {
            "jpg"
        };
        let filename = format!("artwork.{}", img_ext);
        let artwork_path = out_dir.join(&filename);
        fs::write(&artwork_path, &artwork.data)
            .with_context(|| format!("アートワークの書き出しに失敗: {}", artwork_path.display()))?;

        artwork_filename = Some(filename.clone());
        files.push(filename);
        eprintln!("  アートワーク: {}", artwork_path.display());
    }

    let now = Local::now().fixed_offset();
    let updated_at = now.to_rfc3339();

    // now_playing.json を書き出す
    let now_playing = NowPlaying {
        title: meta.title.clone(),
        artist: meta.artist.clone(),
        album: meta.album.clone(),
        artwork: artwork_filename,
        comment: meta.comment.clone(),
        updated_at: updated_at.clone(),
    };

    let np_path = out_dir.join("now_playing.json");
    let np_json = serde_json::to_string_pretty(&now_playing)?;
    fs::write(&np_path, &np_json)
        .with_context(|| format!("now_playing.json の書き出しに失敗: {}", np_path.display()))?;

    // .ready を書き出す (最後に)
    let ready = ReadyManifest { updated_at, files };

    let ready_path = out_dir.join(".ready");
    let ready_json = serde_json::to_string_pretty(&ready)?;
    fs::write(&ready_path, &ready_json)
        .with_context(|| format!(".ready の書き出しに失敗: {}", ready_path.display()))?;

    eprintln!("✅ 出力完了: {}/{}", out.display(), id);
    eprintln!(
        "   {} - {} ({})",
        meta.artist,
        meta.title,
        meta.album.as_deref().unwrap_or("-")
    );

    Ok(())
}

/// DJ プロファイルを書き出す
///
/// 値が画像ファイルパス (png/jpg/jpeg) の場合はコピーして dj-profile.{ext} として出力、
/// それ以外はテキストとして dj-profile.txt に出力する。
fn write_dj_profile(out_dir: &Path, value: &str) -> Result<()> {
    let path = PathBuf::from(value);

    // 既存の dj-profile.* を削除
    for entry in fs::read_dir(out_dir)?.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "dj-profile"
            || name_str == "dj-profile.txt"
            || name_str == "dj-profile.png"
            || name_str == "dj-profile.jpg"
            || name_str == "dj-profile.jpeg"
        {
            fs::remove_file(entry.path())?;
        }
    }

    if path.is_file() {
        // 画像ファイルとして扱う
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if matches!(ext.as_str(), "png" | "jpg" | "jpeg") {
            let dest = out_dir.join(format!("dj-profile.{}", ext));
            fs::copy(&path, &dest).with_context(|| {
                format!("DJ プロファイル画像のコピーに失敗: {}", path.display())
            })?;
            eprintln!("  DJ プロファイル (画像): {}", dest.display());
        } else {
            let dest = out_dir.join("dj-profile.txt");
            fs::write(&dest, value)?;
            eprintln!("  DJ プロファイル (テキスト): {}", value);
        }
    } else {
        let dest = out_dir.join("dj-profile.txt");
        fs::write(&dest, value)?;
        eprintln!("  DJ プロファイル (テキスト): {}", value);
    }

    Ok(())
}
