//! 楽曲ファイルからのタグ読み取り

use std::path::Path;

use anyhow::{Context, Result};
use id3::TagLike;

/// 楽曲から抽出されたメタデータ
pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub comment: Option<String>,
    pub artwork: Option<ArtworkData>,
}

pub struct ArtworkData {
    pub data: Vec<u8>,
    pub mime: String,
}

/// 楽曲ファイルからタグを読み取る
pub fn read_tags(path: &Path) -> Result<TrackMeta> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "mp3" => read_mp3_tags(path),
        "m4a" | "mp4" | "aac" => read_m4a_tags(path),
        _ => anyhow::bail!("未対応のファイル形式: .{}", ext),
    }
}

/// MP3 ファイルから ID3 タグを読み取る
fn read_mp3_tags(path: &Path) -> Result<TrackMeta> {
    let tag = id3::Tag::read_from_path(path)
        .with_context(|| format!("ID3 タグの読み取りに失敗: {}", path.display()))?;

    let title = tag.title().unwrap_or("Unknown Title").to_string();
    let artist = tag.artist().unwrap_or("Unknown Artist").to_string();
    let album = tag.album().map(|s| s.to_string());
    let comment = tag
        .comments()
        .find(|c| c.description.is_empty() || c.description == "Comment")
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
fn read_m4a_tags(path: &Path) -> Result<TrackMeta> {
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
