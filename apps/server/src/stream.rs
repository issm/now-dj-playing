use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::session::{ErrorResponse, SessionEvent, SessionStore};

/// SSE ストリームのクエリパラメータ
#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    /// トークン（EventSource は Authorization ヘッダを送れないため、クエリパラメータでも受け付ける）
    pub token: Option<String>,
}

/// GET /api/sessions/{session_id}/stream
///
/// owner の token、または connect で発行された参加者用 token で購読できる
pub async fn stream(
    State(store): State<Arc<SessionStore>>,
    Path(session_id): Path<String>,
    Query(query): Query<StreamQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // トークン取得: クエリパラメータ優先、なければ Authorization ヘッダ
    let token = query
        .token
        .as_deref()
        .or_else(|| extract_bearer_token(&headers))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "トークンが必要です（クエリパラメータ token または Authorization ヘッダ）"
                        .to_string(),
                }),
            )
        })?;

    // viewer トークンの検証
    let valid_session_id = store.find_session_by_viewer_token(token).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "無効なトークンです".to_string(),
            }),
        )
    })?;

    if valid_session_id != session_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "このセッションへのアクセス権がありません".to_string(),
            }),
        ));
    }

    // broadcast チャネルを subscribe
    // （スナップショットの取得より先に subscribe することで、取りこぼしを防ぐ。
    //   その代わり、参加の瞬間と重なると publisher_joined が重複し得るが、viewer 側は冪等）
    let rx = store.subscribe(&session_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "セッションが見つかりません".to_string(),
            }),
        )
    })?;

    // 接続直後に送るスナップショット:
    // 現在参加中の publisher 全員分の publisher_joined → 最新の楽曲情報
    let snapshot: Vec<Event> = store
        .get_publishers(&session_id)
        .into_iter()
        .map(|p| {
            event_to_sse(SessionEvent::PublisherJoined {
                publisher_id: p.id,
                dj_name: p.dj_name,
                dj_image: p.dj_image,
            })
        })
        .chain(
            store
                .get_last_track(&session_id)
                .map(|track| event_to_sse(SessionEvent::TrackChanged(track))),
        )
        .collect();

    // BroadcastStream に変換し、SSE イベントに変換
    let broadcast_stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => Some(event_to_sse(event)),
        Err(_) => None, // lagged (受信が遅れた) 場合はスキップ
    });

    // スナップショット + broadcast ストリームを結合
    let initial_stream = tokio_stream::iter(snapshot);

    let combined = initial_stream.chain(broadcast_stream);

    let sse = Sse::new(combined.map(Ok::<_, Infallible>))
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(30)));

    Ok(sse)
}

/// SessionEvent を SSE Event に変換する
fn event_to_sse(event: SessionEvent) -> Event {
    let name = match &event {
        SessionEvent::TrackChanged(_) => "track_changed",
        SessionEvent::PublisherJoined { .. } => "publisher_joined",
        SessionEvent::PublisherLeft { .. } => "publisher_left",
        SessionEvent::SessionDestroyed => "session_destroyed",
    };

    Event::default()
        .event(name)
        .data(serde_json::to_string(&event).unwrap_or_default())
}

/// Authorization ヘッダから Bearer トークンを抽出する
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
