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
///
/// dj_image が指定されている場合、dj_name よりも dj_image を優先して DJ プロファイルに使用する。
pub fn publish_local(
    meta: &TrackMeta,
    out: &Path,
    id: &str,
    dj_name: Option<&str>,
    dj_image: Option<&Path>,
) -> Result<()> {
    // 出力先ディレクトリの決定: {out}/{id}/
    let out_dir = out.join(id);
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("出力先ディレクトリの作成に失敗: {}", out_dir.display()))?;

    // DJ プロファイルの書き出し（優先度: 画像 > テキスト）
    if let Some(image_path) = dj_image {
        write_dj_profile_image(&out_dir, image_path)?;
    } else if let Some(name) = dj_name {
        write_dj_profile_text(&out_dir, name)?;
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

/// 既存の dj-profile.* ファイルを削除する
fn clean_dj_profile(out_dir: &Path) -> Result<()> {
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
    Ok(())
}

/// DJ プロファイルを画像ファイルとして書き出す
fn write_dj_profile_image(out_dir: &Path, image_path: &Path) -> Result<()> {
    clean_dj_profile(out_dir)?;

    if !image_path.is_file() {
        eprintln!(
            "  警告: dj_image が見つかりません: {}",
            image_path.display()
        );
        return Ok(());
    }

    let ext = image_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let dest_ext = match ext.as_str() {
        "png" | "jpg" | "jpeg" => ext.as_str(),
        _ => "png",
    };

    let dest = out_dir.join(format!("dj-profile.{}", dest_ext));
    fs::copy(image_path, &dest).with_context(|| {
        format!(
            "DJ プロファイル画像のコピーに失敗: {}",
            image_path.display()
        )
    })?;
    eprintln!("  DJ プロファイル (画像): {}", dest.display());

    Ok(())
}

/// DJ プロファイルをテキストとして書き出す
///
/// 値が画像ファイルパスの場合は画像としてコピーする（後方互換）。
fn write_dj_profile_text(out_dir: &Path, value: &str) -> Result<()> {
    clean_dj_profile(out_dir)?;

    let path = PathBuf::from(value);

    if path.is_file() {
        // 画像ファイルとして扱う（後方互換: dj_name に画像パスが入っているケース）
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
