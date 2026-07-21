use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::session::{ErrorResponse, SessionStore, TrackData};

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub comment: Option<String>,
    pub artwork: Option<String>,
    pub updated_at: String,
}

/// POST /api/publish
pub async fn publish(
    State(store): State<Arc<SessionStore>>,
    headers: HeaderMap,
    Json(body): Json<PublishRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Authorization ヘッダからトークンを取得
    let token = extract_bearer_token(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Authorization ヘッダが必要です".to_string(),
            }),
        )
    })?;

    // トークンから publisher とセッションを特定
    let (session_id, publisher) = store.find_publisher_by_token(token).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "無効なトークンです".to_string(),
            }),
        )
    })?;

    // TrackData を組み立てて publish
    let track = TrackData {
        publisher_id: publisher.id,
        dj_name: publisher.dj_name,
        title: body.title,
        artist: body.artist,
        album: body.album,
        comment: body.comment,
        artwork: body.artwork,
        updated_at: body.updated_at,
    };

    store.publish_track(&session_id, track);

    Ok(StatusCode::NO_CONTENT)
}

/// Authorization ヘッダから Bearer トークンを抽出する
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
