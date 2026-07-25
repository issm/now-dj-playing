use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use std::sync::Arc;

use crate::session::{ErrorResponse, LeaveError, SessionStore};

/// POST /api/sessions/{session_id}/leave
pub async fn leave(
    State(store): State<Arc<SessionStore>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
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
    let (token_session_id, publisher) = store.find_publisher_by_token(token).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "無効なトークンです".to_string(),
            }),
        )
    })?;

    // パスのセッション ID とトークンのセッション ID が一致するか検証
    if token_session_id != session_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "このセッションへのアクセス権がありません".to_string(),
            }),
        ));
    }

    // セッションから publisher を離脱
    store
        .leave(&session_id, &publisher.id)
        .map_err(|e| match e {
            LeaveError::SessionNotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "セッションが見つかりません".to_string(),
                }),
            ),
            LeaveError::PublisherNotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Publisher が見つかりません".to_string(),
                }),
            ),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Authorization ヘッダから Bearer トークンを抽出する
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
