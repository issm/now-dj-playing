use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use id3::TagLike;
use serde::Serialize;

#[derive(Parser)]
#[command(name = "ndp-publish")]
#[command(about = "楽曲ファイルからタグ・アートワークを抽出し、共有ディレクトリに出力する")]
struct Cli {
    /// 楽曲ファイルのパス (mp3, m4a)
    #[arg(short, long)]
    file: PathBuf,

    /// 出力先ベースディレクトリ
    #[arg(short, long)]
    out: PathBuf,

    /// DJ ディレクトリ名 (out 配下に作成される)
    #[arg(long, default_value = "dj-000")]
    id: String,

    /// DJ 名 (テキスト) またはロゴ画像パス (png/jpg/jpeg)
    /// テキストを渡すと dj-profile.txt、画像パスを渡すと dj-profile.{ext} として出力
    #[arg(long)]
    dj_name: Option<String>,
}

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

/// 楽曲から抽出されたメタデータ
struct TrackMeta {
    title: String,
    artist: String,
    album: Option<String>,
    comment: Option<String>,
    artwork: Option<ArtworkData>,
}

struct ArtworkData {
    data: Vec<u8>,
    mime: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 楽曲ファイルの存在確認
    if !cli.file.is_file() {
        anyhow::bail!("楽曲ファイルが見つかりません: {}", cli.file.display());
    }

    // 出力先ディレクトリの決定: {out}/{id}/
    let out_dir = cli.out.join(&cli.id);
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("出力先ディレクトリの作成に失敗: {}", out_dir.display()))?;

    // DJ プロファイルの書き出し
    if let Some(dj_name) = &cli.dj_name {
        write_dj_profile(&out_dir, dj_name)?;
    }

    // タグを読み取る
    let ext = cli
        .file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let meta = match ext.as_str() {
        "mp3" => read_mp3_tags(&cli.file)?,
        "m4a" | "mp4" | "aac" => read_m4a_tags(&cli.file)?,
        _ => anyhow::bail!("未対応のファイル形式: .{}", ext),
    };

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

    eprintln!("✅ 出力完了: {}/{}", cli.out.display(), cli.id);
    eprintln!(
        "   {} - {} ({})",
        meta.artist,
        meta.title,
        meta.album.as_deref().unwrap_or("-")
    );

    Ok(())
}

/// MP3 ファイルから ID3 タグを読み取る
fn read_mp3_tags(path: &PathBuf) -> Result<TrackMeta> {
    let tag = id3::Tag::read_from_path(path)
        .with_context(|| format!("ID3 タグの読み取りに失敗: {}", path.display()))?;

    let title = tag.title().unwrap_or("Unknown Title").to_string();
    let artist = tag.artist().unwrap_or("Unknown Artist").to_string();
    let album = tag.album().map(|s| s.to_string());
    let comment = tag
        .comments()
        .next()
        .map(|c| c.text.clone())
        .filter(|s| !s.is_empty());

    let artwork = tag.pictures().next().map(|pic| ArtworkData {
        data: pic.data.clone(),
        mime: pic.mime_type.clone(),
    });

    Ok(TrackMeta {
        title,
        artist,
        album,
        comment,
        artwork,
    })
}

/// M4A ファイルからメタデータを読み取る
fn read_m4a_tags(path: &PathBuf) -> Result<TrackMeta> {
    let tag = mp4ameta::Tag::read_from_path(path)
        .with_context(|| format!("M4A タグの読み取りに失敗: {}", path.display()))?;

    let title = tag.title().unwrap_or("Unknown Title").to_string();
    let artist = tag.artist().unwrap_or("Unknown Artist").to_string();
    let album = tag.album().map(|s| s.to_string());
    let comment = tag
        .comment()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    let artwork = tag.artworks().next().map(|art| ArtworkData {
        data: art.data.to_vec(),
        mime: match art.fmt {
            mp4ameta::ImgFmt::Png => "image/png".to_string(),
            mp4ameta::ImgFmt::Jpeg => "image/jpeg".to_string(),
            mp4ameta::ImgFmt::Bmp => "image/bmp".to_string(),
        },
    });

    Ok(TrackMeta {
        title,
        artist,
        album,
        comment,
        artwork,
    })
}

/// DJ プロファイルを書き出す
///
/// 値が画像ファイルパス (png/jpg/jpeg) の場合はコピーして dj-profile.{ext} として出力、
/// それ以外はテキストとして dj-profile.txt に出力する。
fn write_dj_profile(out_dir: &PathBuf, value: &str) -> Result<()> {
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
