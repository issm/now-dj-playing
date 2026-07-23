//! web モード: ndp-server への HTTP 送信

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine;
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::tags::TrackMeta;

/// セッショントークン情報（ファイルに永続化する）
#[derive(Debug, Serialize, Deserialize)]
struct SessionToken {
    session_id: String,
    code: String,
    publisher_id: String,
    token: String,
}

/// join API のリクエスト
#[derive(Serialize)]
struct JoinRequest {
    code: String,
    dj_name: String,
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

    let token_path = config.session_token_path().ok_or_else(|| {
        anyhow::anyhow!(
            "設定ファイルのパスが不明なためトークンファイルを保存できません。-c で設定ファイルを指定してください"
        )
    })?;

    // セッショントークンの取得（既存 or 新規 join）
    let session_token = resolve_session_token(&endpoint_url, &token_path, dj_name, code)?;

    // アートワークを Base64 Data URI にエンコード
    let artwork_data_uri = meta.artwork.as_ref().map(|art| {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&art.data);
        format!("data:{};base64,{}", art.mime, b64)
    });

    let now = Local::now().fixed_offset();
    let updated_at = now.to_rfc3339();

    // publish リクエストを送信
    let publish_url = format!(
        "{}/sessions/{}/publish",
        endpoint_url.trim_end_matches('/'),
        session_token.session_id
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
        .header("Authorization", format!("Bearer {}", session_token.token))
        .json(&body)
        .send()
        .with_context(|| format!("publish リクエストの送信に失敗: {}", publish_url))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().unwrap_or_default();
        anyhow::bail!(
            "publish に失敗 (HTTP {}): {}",
            status.as_u16(),
            error_text
        );
    }

    eprintln!("✅ 送信完了 (web): session={}", session_token.session_id);
    eprintln!(
        "   {} - {} ({})",
        meta.artist,
        meta.title,
        meta.album.as_deref().unwrap_or("-")
    );

    Ok(())
}

/// セッショントークンを解決する
///
/// - トークンファイルが存在し、code 指定なしまたは同じ code → 再利用
/// - それ以外 → 新規 join
fn resolve_session_token(
    endpoint_url: &str,
    token_path: &Path,
    dj_name: &str,
    code: Option<&str>,
) -> Result<SessionToken> {
    // 既存トークンの読み込み試行
    if let Ok(content) = fs::read_to_string(token_path) {
        if let Ok(existing) = serde_json::from_str::<SessionToken>(&content) {
            // --code 未指定、または同じ code なら再利用
            match code {
                None => {
                    eprintln!("  既存セッション再利用: {}", existing.session_id);
                    return Ok(existing);
                }
                Some(c) if c == existing.code => {
                    eprintln!("  既存セッション再利用: {}", existing.session_id);
                    return Ok(existing);
                }
                _ => {
                    // 新しい code が指定された → 再 join
                }
            }
        }
    }

    // 新規 join
    let join_code = code.ok_or_else(|| {
        anyhow::anyhow!(
            "セッションコードが必要です。--code で 6 桁コードを指定してください"
        )
    })?;

    let join_url = format!("{}/sessions/join", endpoint_url.trim_end_matches('/'));
    let join_body = JoinRequest {
        code: join_code.to_string(),
        dj_name: dj_name.to_string(),
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

    let join_resp: JoinResponse = response
        .json()
        .context("join レスポンスのパースに失敗")?;

    let session_token = SessionToken {
        session_id: join_resp.session_id.clone(),
        code: join_code.to_string(),
        publisher_id: join_resp.publisher_id,
        token: join_resp.token,
    };

    // トークンファイルに保存
    let token_json = serde_json::to_string_pretty(&session_token)?;
    fs::write(token_path, &token_json).with_context(|| {
        format!(
            "トークンファイルの保存に失敗: {}",
            token_path.display()
        )
    })?;

    eprintln!("  セッション参加: {}", join_resp.session_id);

    Ok(session_token)
}
