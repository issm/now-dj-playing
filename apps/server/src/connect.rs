use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::session::{ConnectError, DisconnectError, ErrorResponse, SessionStore};

#[derive(Debug, Deserialize)]
pub struct ConnectSessionRequest {
    /// view 用コード（`create` のレスポンスの `roles.viewer.code`）
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectSessionResponse {
    pub session_id: String,
    /// 参加者用 token（購読のみ可能）。フィールド名は既存クライアントとの一貫性のため `viewer_token`
    pub viewer_token: String,
    pub event_name: Option<String>,
}

/// POST /api/sessions/connect
pub async fn connect_session(
    State(store): State<Arc<SessionStore>>,
    Json(body): Json<ConnectSessionRequest>,
) -> Result<Json<ConnectSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    match store.connect_by_code(&body.code) {
        Ok((session, token)) => Ok(Json(ConnectSessionResponse {
            session_id: session.id,
            viewer_token: token,
            event_name: session.event_name,
        })),
        Err(ConnectError::InvalidCode) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "無効なセッションコードです".to_string(),
            }),
        )),
    }
}

/// POST /api/sessions/{session_id}/disconnect
///
/// Authorization: Bearer {参加者用 token}
pub async fn disconnect_session(
    State(store): State<Arc<SessionStore>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Authorization ヘッダが必要です".to_string(),
            }),
        )
    })?;

    match store.disconnect(&session_id, token) {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(DisconnectError::SessionNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "セッションが見つかりません".to_string(),
            }),
        )),
        Err(DisconnectError::InvalidToken) => Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "無効なトークンです".to_string(),
            }),
        )),
    }
}

/// Authorization ヘッダから Bearer トークンを抽出する
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
