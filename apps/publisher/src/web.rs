//! web モード: ndp-server への HTTP 送信

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine;
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::tags::TrackMeta;

/// セッション情報（ファイルに永続化する）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionInfo {
    session_id: String,
    publisher_id: String,
    token: String,
}

/// join API のリクエスト
#[derive(Serialize)]
struct JoinRequest {
    code: String,
    dj_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dj_image: Option<String>,
}

/// join API のレスポンス
#[derive(Deserialize)]
struct JoinResponse {
    session_id: String,
    publisher_id: String,
    token: String,
}

/// publish API のリクエスト
#[derive(Serialize)]
struct PublishRequest {
    title: String,
    artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artwork: Option<String>,
    updated_at: String,
}

/// セッションから離脱する (-L, --leave)
pub fn leave(config: &AppConfig) -> Result<()> {
    let endpoint_url = config.web_endpoint_url().ok_or_else(|| {
        anyhow::anyhow!(
            "エンドポイント URL が未指定です。設定ファイルの web.endpoint_url を設定してください"
        )
    })?;

    let session_path = config.session_file_path().ok_or_else(|| {
        anyhow::anyhow!(
            "セッションファイルの保存先を特定できません。-c で設定ファイルを指定するか、NDP_PUBLISH_SESSION_DIR を設定してください"
        )
    })?;

    // セッションファイルの読み込み
    let content = fs::read_to_string(&session_path).with_context(|| {
        format!(
            "セッションファイルが見つかりません: {}",
            session_path.display()
        )
    })?;
    let session: SessionInfo =
        serde_json::from_str(&content).context("セッションファイルのパースに失敗")?;

    // leave API 呼び出し
    let leave_url = format!(
        "{}/sessions/{}/leave",
        endpoint_url.trim_end_matches('/'),
        session.session_id
    );

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&leave_url)
        .header("Authorization", format!("Bearer {}", session.token))
        .send()
        .with_context(|| format!("leave リクエストの送信に失敗: {}", leave_url))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().unwrap_or_default();
        anyhow::bail!("leave に失敗 (HTTP {}): {}", status.as_u16(), error_text);
    }

    // セッションファイルを削除
    fs::remove_file(&session_path)
        .with_context(|| format!("セッションファイルの削除に失敗: {}", session_path.display()))?;

    eprintln!("✅ セッション離脱完了");
    eprintln!("   session_id:   {}", session.session_id);
    eprintln!("   publisher_id: {}", session.publisher_id);

    Ok(())
}

/// join のみ実行して終了する (-J, --join-only)
pub fn join_only(config: &AppConfig, dj_name: &str, code: Option<&str>) -> Result<()> {
    let endpoint_url = config.web_endpoint_url().ok_or_else(|| {
        anyhow::anyhow!(
            "エンドポイント URL が未指定です。設定ファイルの web.endpoint_url を設定してください"
        )
    })?;

    let session_path = config.session_file_path().ok_or_else(|| {
        anyhow::anyhow!(
            "セッションファイルの保存先を特定できません。-c で設定ファイルを指定するか、NDP_PUBLISH_SESSION_DIR を設定してください"
        )
    })?;

    let code = code.ok_or_else(|| {
        anyhow::anyhow!("セッションコードが必要です。-C で 6 桁コードを指定してください")
    })?;

    // dj_image を読み込み・リサイズ
    let dj_image_data_uri = load_dj_image_data_uri(config)?;

    let session = do_join(&endpoint_url, dj_name, code, dj_image_data_uri.as_deref())?;
    save_session_file(&session_path, &session)?;

    eprintln!("✅ セッション参加完了");
    eprintln!("   session_id:   {}", session.session_id);
    eprintln!("   publisher_id: {}", session.publisher_id);
    eprintln!("   セッションファイル: {}", session_path.display());

    Ok(())
}

/// web モードで楽曲情報を ndp-server に送信する
pub fn publish_web(
    config: &AppConfig,
    meta: &TrackMeta,
    dj_name: &str,
    code: Option<&str>,
) -> Result<()> {
    let endpoint_url = config.web_endpoint_url().ok_or_else(|| {
        anyhow::anyhow!(
            "エンドポイント URL が未指定です。設定ファイルの web.endpoint_url を設定してください"
        )
    })?;

    let session_path = config.session_file_path().ok_or_else(|| {
        anyhow::anyhow!(
            "セッションファイルの保存先を特定できません。-c で設定ファイルを指定するか、NDP_PUBLISH_SESSION_DIR を設定してください"
        )
    })?;

    // dj_image を読み込み・リサイズ（再 join 時に使用）
    let dj_image_data_uri = load_dj_image_data_uri(config)?;

    // セッション情報の取得
    let session = resolve_session(
        &endpoint_url,
        &session_path,
        dj_name,
        code,
        dj_image_data_uri.as_deref(),
    )?;

    // publish 実行
    let publish_result = do_publish(&endpoint_url, &session, meta);

    match publish_result {
        Ok(()) => {
            // 成功
        }
        Err(ref e) if is_unauthorized_error(e) => {
            // 401: 再 join を試行
            let rejoin_code = code.ok_or_else(|| {
                anyhow::anyhow!(
                    "セッションが無効です (401)。-C で新しいコードを指定して再参加してください"
                )
            })?;
            eprintln!("  セッション無効 (401)、再参加を試行...");
            let new_session = do_join(
                &endpoint_url,
                dj_name,
                rejoin_code,
                dj_image_data_uri.as_deref(),
            )?;
            save_session_file(&session_path, &new_session)?;

            // publish 再試行
            do_publish(&endpoint_url, &new_session, meta)?;
        }
        Err(e) => return Err(e),
    }

    eprintln!("✅ 送信完了 (web): session={}", session.session_id);
    eprintln!(
        "   {} - {} ({})",
        meta.artist,
        meta.title,
        meta.album.as_deref().unwrap_or("-")
    );

    Ok(())
}

/// セッション情報を解決する
///
/// - セッションファイルが存在する → その情報を返す
/// - 存在しない → join を試行してセッションファイルを作成
fn resolve_session(
    endpoint_url: &str,
    session_path: &Path,
    dj_name: &str,
    code: Option<&str>,
    dj_image: Option<&str>,
) -> Result<SessionInfo> {
    // 既存セッションファイルの読み込み試行
    if let Ok(content) = fs::read_to_string(session_path) {
        if let Ok(existing) = serde_json::from_str::<SessionInfo>(&content) {
            eprintln!("  既存セッション: {}", existing.session_id);
            return Ok(existing);
        }
    }

    // セッションファイル不在 → join
    let join_code = code.ok_or_else(|| {
        anyhow::anyhow!("セッションコードが必要です。-C で 6 桁コードを指定してください")
    })?;

    let session = do_join(endpoint_url, dj_name, join_code, dj_image)?;
    save_session_file(session_path, &session)?;

    Ok(session)
}

/// join API を呼び出す
fn do_join(
    endpoint_url: &str,
    dj_name: &str,
    code: &str,
    dj_image: Option<&str>,
) -> Result<SessionInfo> {
    let join_url = format!("{}/sessions/join", endpoint_url.trim_end_matches('/'));
    let join_body = JoinRequest {
        code: code.to_string(),
        dj_name: dj_name.to_string(),
        dj_image: dj_image.map(|s| s.to_string()),
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&join_url)
        .json(&join_body)
        .send()
        .with_context(|| format!("join リクエストの送信に失敗: {}", join_url))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().unwrap_or_default();
        anyhow::bail!("join に失敗 (HTTP {}): {}", status.as_u16(), error_text);
    }

    let join_resp: JoinResponse = response.json().context("join レスポンスのパースに失敗")?;

    eprintln!("  セッション参加: {}", join_resp.session_id);

    Ok(SessionInfo {
        session_id: join_resp.session_id,
        publisher_id: join_resp.publisher_id,
        token: join_resp.token,
    })
}

/// publish API を呼び出す
fn do_publish(endpoint_url: &str, session: &SessionInfo, meta: &TrackMeta) -> Result<()> {
    // アートワークをリサイズ + Base64 Data URI にエンコード
    let artwork_data_uri = meta
        .artwork
        .as_ref()
        .map(|art| resize_and_encode_artwork(art))
        .transpose()?;

    let now = Local::now().fixed_offset();
    let updated_at = now.to_rfc3339();

    let publish_url = format!(
        "{}/sessions/{}/publish",
        endpoint_url.trim_end_matches('/'),
        session.session_id
    );

    let body = PublishRequest {
        title: meta.title.clone(),
        artist: meta.artist.clone(),
        album: meta.album.clone(),
        comment: meta.comment.clone(),
        artwork: artwork_data_uri,
        updated_at,
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&publish_url)
        .header("Authorization", format!("Bearer {}", session.token))
        .json(&body)
        .send()
        .with_context(|| format!("publish リクエストの送信に失敗: {}", publish_url))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().unwrap_or_default();
        anyhow::bail!("publish に失敗 (HTTP {}): {}", status.as_u16(), error_text);
    }

    Ok(())
}

/// セッションファイルを保存する
fn save_session_file(path: &Path, session: &SessionInfo) -> Result<()> {
    let json = serde_json::to_string_pretty(session)?;
    fs::write(path, &json)
        .with_context(|| format!("セッションファイルの保存に失敗: {}", path.display()))?;
    Ok(())
}

/// エラーが 401 Unauthorized かどうか判定する
fn is_unauthorized_error(e: &anyhow::Error) -> bool {
    e.to_string().contains("HTTP 401")
}

/// 画像の最大辺 (px)
const IMAGE_MAX_DIMENSION: u32 = 800;

/// 設定ファイルの dj_image パスから画像を読み込み、800x800 にリサイズして Data URI を返す
///
/// dj_image が未設定の場合は None を返す。
fn load_dj_image_data_uri(config: &AppConfig) -> Result<Option<String>> {
    let image_path = match config.dj_image_path() {
        Some(p) => p,
        None => return Ok(None),
    };

    if !image_path.is_file() {
        eprintln!(
            "  警告: dj_image が見つかりません: {}",
            image_path.display()
        );
        return Ok(None);
    }

    let data_uri = resize_and_encode_image_file(&image_path)?;
    Ok(Some(data_uri))
}

/// 画像ファイルを読み込み、800x800 に収まるようリサイズして Base64 Data URI を返す
fn resize_and_encode_image_file(path: &Path) -> Result<String> {
    use image::ImageReader;

    let img = ImageReader::open(path)
        .with_context(|| format!("画像ファイルを開けません: {}", path.display()))?
        .decode()
        .with_context(|| format!("画像のデコードに失敗: {}", path.display()))?;

    let (w, h) = (img.width(), img.height());

    let resized = if w > IMAGE_MAX_DIMENSION || h > IMAGE_MAX_DIMENSION {
        img.resize(
            IMAGE_MAX_DIMENSION,
            IMAGE_MAX_DIMENSION,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };

    // 拡張子で出力形式を決定
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (format, mime) = if ext == "png" {
        (image::ImageFormat::Png, "image/png")
    } else {
        (image::ImageFormat::Jpeg, "image/jpeg")
    };

    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    resized
        .write_to(&mut cursor, format)
        .context("画像の再エンコードに失敗")?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);

    eprintln!(
        "  DJ 画像 (送信): {} {}x{} ({})",
        if ext == "png" { "PNG" } else { "JPEG" },
        resized.width(),
        resized.height(),
        format_size(buf.len())
    );

    Ok(format!("data:{};base64,{}", mime, b64))
}

/// アートワークを 800x800 に収まるようリサイズし、Base64 Data URI を返す
/// 元の画像形式 (JPEG/PNG) を維持する
fn resize_and_encode_artwork(art: &crate::tags::ArtworkData) -> Result<String> {
    use image::ImageReader;
    use std::io::Cursor;

    let img = ImageReader::new(Cursor::new(&art.data))
        .with_guessed_format()
        .context("アートワーク画像フォーマットの判定に失敗")?
        .decode()
        .context("アートワーク画像のデコードに失敗")?;

    let (w, h) = (img.width(), img.height());

    let resized = if w > IMAGE_MAX_DIMENSION || h > IMAGE_MAX_DIMENSION {
        img.resize(
            IMAGE_MAX_DIMENSION,
            IMAGE_MAX_DIMENSION,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };

    // 元の形式を維持してエンコード
    let (format, mime) = if art.mime.contains("png") {
        (image::ImageFormat::Png, "image/png")
    } else {
        (image::ImageFormat::Jpeg, "image/jpeg")
    };

    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    resized
        .write_to(&mut cursor, format)
        .context("アートワーク画像の再エンコードに失敗")?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    let ext_label = if art.mime.contains("png") {
        "PNG"
    } else {
        "JPEG"
    };

    eprintln!(
        "  アートワーク (送信): {} {}x{} ({})",
        ext_label,
        resized.width(),
        resized.height(),
        format_size(buf.len())
    );

    Ok(format!("data:{};base64,{}", mime, b64))
}

/// バイト数を適切な単位で表示する
fn format_size(bytes: usize) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    }
}
