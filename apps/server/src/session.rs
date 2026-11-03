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
    /// セッションが破棄された（破棄の直前に配信する）
    #[serde(rename = "session_destroyed")]
    SessionDestroyed,
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
    /// publisher が join するためのコード（publish 権限）
    pub code: String,
    /// viewer が connect するためのコード（view 権限）。`code` とは別の値
    pub viewer_code: String,
    pub event_name: Option<String>,
    /// セッション作成者（owner）の token。購読とセッション破棄ができる
    pub viewer_token: String,
    /// connect した viewer に発行した token の一覧。購読のみできる
    pub connected_viewers: Vec<String>,
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
        let viewer_code = generate_code_distinct_from(&code);
        let viewer_token = format!("vt_{}", Uuid::new_v4().simple());

        let session = Session {
            id: id.clone(),
            code,
            viewer_code,
            event_name,
            viewer_token,
            connected_viewers: Vec::new(),
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

    /// view 用コードでセッションを検索し、参加者用 token を発行する
    ///
    /// 呼び出しごとに新しい token を発行し、`connected_viewers` に登録する
    pub fn connect_by_code(&self, code: &str) -> Result<(Session, String), ConnectError> {
        let mut sessions = self.sessions.lock().unwrap();

        let session = sessions
            .values_mut()
            .find(|s| s.viewer_code == code)
            .ok_or(ConnectError::InvalidCode)?;

        let token = format!("vt_{}", Uuid::new_v4().simple());
        session.connected_viewers.push(token.clone());

        Ok((session.clone(), token))
    }

    /// 参加者用 token を失効させる（以降その token での購読はできない）
    pub fn disconnect(&self, session_id: &str, token: &str) -> Result<(), DisconnectError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or(DisconnectError::SessionNotFound)?;

        let pos = session
            .connected_viewers
            .iter()
            .position(|t| t == token)
            .ok_or(DisconnectError::InvalidToken)?;

        session.connected_viewers.remove(pos);
        Ok(())
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

    /// 購読用トークン（owner の token、または参加者用 token）からセッション ID を特定する
    pub fn find_session_by_viewer_token(&self, token: &str) -> Option<String> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .values()
            .find(|s| s.viewer_token == token || s.connected_viewers.iter().any(|t| t == token))
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

    /// セッションに参加中の publisher 一覧を取得する（SSE 接続時のスナップショット用）
    pub fn get_publishers(&self, session_id: &str) -> Vec<Publisher> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .get(session_id)
            .map(|s| s.publishers.clone())
            .unwrap_or_default()
    }

    /// セッションを破棄する（owner の token による認証付き）
    ///
    /// 破棄の直前に `session_destroyed` を配信し、購読中の viewer に終了を伝える
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

        // session_destroyed を配信してから broadcast チャネルを削除する
        // （送信済みのイベントは receiver が読み切るまで保持され、その後ストリームが閉じる）
        let mut channels = self.channels.lock().unwrap();
        if let Some(ch) = channels.remove(session_id) {
            let _ = ch.tx.send(SessionEvent::SessionDestroyed);
        }

        Ok(())
    }
}

/// join 時のエラー
#[derive(Debug)]
pub enum JoinError {
    InvalidCode,
}

/// connect 時のエラー
#[derive(Debug)]
pub enum ConnectError {
    InvalidCode,
}

/// disconnect 時のエラー
#[derive(Debug)]
pub enum DisconnectError {
    SessionNotFound,
    /// 参加者用 token として登録されていない（owner の token、別セッションの token を含む）
    InvalidToken,
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

/// 指定したコードとは異なる 6 桁のコードを生成する
fn generate_code_distinct_from(other: &str) -> String {
    loop {
        let code = generate_code();
        if code != other {
            return code;
        }
    }
}

// --- API ---

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub event_name: Option<String>,
}

/// ロールごとの参加情報
#[derive(Debug, Serialize)]
pub struct RoleAccess {
    pub code: String,
}

/// ロール名をキーとする参加情報
#[derive(Debug, Serialize)]
pub struct Roles {
    pub publisher: RoleAccess,
    pub viewer: RoleAccess,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    /// publisher 用コード（過去互換。`roles.publisher.code` と同じ値）
    pub code: String,
    pub event_name: Option<String>,
    /// owner の token（過去互換の名前。`roles.viewer` とは無関係）
    pub viewer_token: String,
    pub roles: Roles,
}

/// POST /api/sessions/create
pub async fn create_session(
    State(store): State<Arc<SessionStore>>,
    Json(body): Json<CreateSessionRequest>,
) -> (StatusCode, Json<CreateSessionResponse>) {
    let session = store.create(body.event_name);

    let response = CreateSessionResponse {
        session_id: session.id,
        roles: Roles {
            publisher: RoleAccess {
                code: session.code.clone(),
            },
            viewer: RoleAccess {
                code: session.viewer_code,
            },
        },
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
/// Authorization: Bearer {viewer_token}（owner の token のみ許可）
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
