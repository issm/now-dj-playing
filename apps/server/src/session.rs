use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;

/// SSE で配信するイベント
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SessionEvent {
    #[serde(rename = "track_changed")]
    TrackChanged(TrackData),
    #[serde(rename = "publisher_joined")]
    PublisherJoined {
        publisher_id: String,
        dj_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dj_image: Option<String>,
    },
    #[serde(rename = "publisher_left")]
    PublisherLeft {
        publisher_id: String,
        dj_name: String,
    },
}

/// 楽曲情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackData {
    pub publisher_id: String,
    pub dj_name: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork: Option<String>,
    pub updated_at: String,
}

/// セッション情報
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub code: String,
    pub event_name: Option<String>,
    pub viewer_token: String,
    pub publishers: Vec<Publisher>,
    /// 最新の楽曲情報（viewer 途中接続時に即送信用）
    pub last_track: Option<TrackData>,
}

/// publisher 情報
#[derive(Debug, Clone)]
pub struct Publisher {
    pub id: String,
    pub dj_name: String,
    pub dj_image: Option<String>,
    pub token: String,
}

/// セッションごとの broadcast チャネル
struct SessionChannel {
    tx: broadcast::Sender<SessionEvent>,
}

/// インメモリのセッションストア
pub struct SessionStore {
    sessions: Mutex<HashMap<String, Session>>,
    channels: Mutex<HashMap<String, SessionChannel>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// セッションを作成して保存する
    pub fn create(&self, event_name: Option<String>) -> Session {
        let id = Uuid::new_v4().to_string();
        let code = generate_code();
        let viewer_token = format!("vt_{}", Uuid::new_v4().simple());

        let session = Session {
            id: id.clone(),
            code,
            event_name,
            viewer_token,
            publishers: Vec::new(),
            last_track: None,
        };

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(id.clone(), session.clone());

        // broadcast チャネルを作成（バッファ 16）
        let (tx, _) = broadcast::channel(16);
        let mut channels = self.channels.lock().unwrap();
        channels.insert(id, SessionChannel { tx });

        session
    }

    /// コードでセッションを検索し、publisher を追加する
    pub fn join_by_code(
        &self,
        code: &str,
        dj_name: &str,
        dj_image: Option<&str>,
    ) -> Result<(Session, Publisher), JoinError> {
        let mut sessions = self.sessions.lock().unwrap();

        let session = sessions
            .values_mut()
            .find(|s| s.code == code)
            .ok_or(JoinError::InvalidCode)?;

        let publisher_id = format!("pub_{}", Uuid::new_v4().simple());
        let token = format!("pt_{}", Uuid::new_v4().simple());

        let publisher = Publisher {
            id: publisher_id,
            dj_name: dj_name.to_string(),
            dj_image: dj_image.map(|s| s.to_string()),
            token,
        };

        session.publishers.push(publisher.clone());

        // publisher_joined イベントを送信
        let channels = self.channels.lock().unwrap();
        if let Some(ch) = channels.get(&session.id) {
            let _ = ch.tx.send(SessionEvent::PublisherJoined {
                publisher_id: publisher.id.clone(),
                dj_name: publisher.dj_name.clone(),
                dj_image: publisher.dj_image.clone(),
            });
        }

        Ok((session.clone(), publisher))
    }

    /// トークンから publisher とセッション ID を特定する
    pub fn find_publisher_by_token(&self, token: &str) -> Option<(String, Publisher)> {
        let sessions = self.sessions.lock().unwrap();
        for session in sessions.values() {
            if let Some(pub_info) = session.publishers.iter().find(|p| p.token == token) {
                return Some((session.id.clone(), pub_info.clone()));
            }
        }
        None
    }

    /// viewer トークンからセッション ID を特定する
    pub fn find_session_by_viewer_token(&self, token: &str) -> Option<String> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .values()
            .find(|s| s.viewer_token == token)
            .map(|s| s.id.clone())
    }

    /// 楽曲情報を publish し、SSE で配信する
    pub fn publish_track(&self, session_id: &str, track: TrackData) {
        // last_track を更新
        {
            let mut sessions = self.sessions.lock().unwrap();
            if let Some(session) = sessions.get_mut(session_id) {
                session.last_track = Some(track.clone());
            }
        }

        // broadcast で配信
        let channels = self.channels.lock().unwrap();
        if let Some(ch) = channels.get(session_id) {
            let _ = ch.tx.send(SessionEvent::TrackChanged(track));
        }
    }

    /// publisher をセッションから離脱させ、SSE で publisher_left イベントを配信する
    pub fn leave(&self, session_id: &str, publisher_id: &str) -> Result<Publisher, LeaveError> {
        let removed_publisher = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or(LeaveError::SessionNotFound)?;

            let pos = session
                .publishers
                .iter()
                .position(|p| p.id == publisher_id)
                .ok_or(LeaveError::PublisherNotFound)?;

            session.publishers.remove(pos)
        };

        // publisher_left イベントを配信
        let channels = self.channels.lock().unwrap();
        if let Some(ch) = channels.get(session_id) {
            let _ = ch.tx.send(SessionEvent::PublisherLeft {
                publisher_id: removed_publisher.id.clone(),
                dj_name: removed_publisher.dj_name.clone(),
            });
        }

        Ok(removed_publisher)
    }

    /// SSE 用の receiver を取得する
    pub fn subscribe(&self, session_id: &str) -> Option<broadcast::Receiver<SessionEvent>> {
        let channels = self.channels.lock().unwrap();
        channels.get(session_id).map(|ch| ch.tx.subscribe())
    }

    /// セッションの最新楽曲情報を取得する
    pub fn get_last_track(&self, session_id: &str) -> Option<TrackData> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).and_then(|s| s.last_track.clone())
    }

    /// セッションを破棄する（viewer_token による認証付き）
    pub fn destroy(&self, session_id: &str, viewer_token: &str) -> Result<(), DestroyError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or(DestroyError::SessionNotFound)?;

        if session.viewer_token != viewer_token {
            return Err(DestroyError::Unauthorized);
        }

        sessions.remove(session_id);
        drop(sessions);

        // broadcast チャネルも削除（receiver 側は SendError になって自然に切断される）
        let mut channels = self.channels.lock().unwrap();
        channels.remove(session_id);

        Ok(())
    }
}

/// join 時のエラー
#[derive(Debug)]
pub enum JoinError {
    InvalidCode,
}

/// leave 時のエラー
#[derive(Debug)]
pub enum LeaveError {
    SessionNotFound,
    PublisherNotFound,
}

/// destroy 時のエラー
#[derive(Debug)]
pub enum DestroyError {
    SessionNotFound,
    Unauthorized,
}

/// 0埋め6桁のセッションコードを生成する
fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    let n: u32 = rng.gen_range(0..1_000_000);
    format!("{:06}", n)
}

// --- API ---

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub event_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub code: String,
    pub event_name: Option<String>,
    pub viewer_token: String,
}

/// POST /api/sessions/create
pub async fn create_session(
    State(store): State<Arc<SessionStore>>,
    Json(body): Json<CreateSessionRequest>,
) -> (StatusCode, Json<CreateSessionResponse>) {
    let session = store.create(body.event_name);

    let response = CreateSessionResponse {
        session_id: session.id,
        code: session.code,
        event_name: session.event_name,
        viewer_token: session.viewer_token,
    };

    (StatusCode::CREATED, Json(response))
}

#[derive(Debug, Deserialize)]
pub struct JoinSessionRequest {
    pub code: String,
    pub dj_name: String,
    pub dj_image: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JoinSessionResponse {
    pub session_id: String,
    pub publisher_id: String,
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// POST /api/sessions/join
pub async fn join_session(
    State(store): State<Arc<SessionStore>>,
    Json(body): Json<JoinSessionRequest>,
) -> Result<Json<JoinSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    match store.join_by_code(&body.code, &body.dj_name, body.dj_image.as_deref()) {
        Ok((session, publisher)) => Ok(Json(JoinSessionResponse {
            session_id: session.id,
            publisher_id: publisher.id,
            token: publisher.token,
        })),
        Err(JoinError::InvalidCode) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "無効なセッションコードです".to_string(),
            }),
        )),
    }
}

/// DELETE /api/sessions/{session_id}
///
/// Authorization: Bearer {viewer_token}
pub async fn destroy_session(
    State(store): State<Arc<SessionStore>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Authorization ヘッダから Bearer トークンを抽出
    let viewer_token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Authorization ヘッダが必要です".to_string(),
                }),
            )
        })?;

    match store.destroy(&session_id, viewer_token) {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(DestroyError::SessionNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "セッションが見つかりません".to_string(),
            }),
        )),
        Err(DestroyError::Unauthorized) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "このセッションを破棄する権限がありません".to_string(),
            }),
        )),
    }
}
