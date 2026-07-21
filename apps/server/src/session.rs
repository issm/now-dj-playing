use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// セッション情報
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub code: String,
    pub event_name: Option<String>,
    pub viewer_token: String,
    /// 参加中の publisher 一覧
    pub publishers: Vec<Publisher>,
}

/// publisher 情報
#[derive(Debug, Clone)]
pub struct Publisher {
    pub id: String,
    pub dj_name: String,
    pub token: String,
}

/// インメモリのセッションストア
#[derive(Debug)]
pub struct SessionStore {
    sessions: Mutex<HashMap<String, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
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
        };

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(id, session.clone());
        session
    }

    /// コードでセッションを検索し、publisher を追加する
    pub fn join_by_code(&self, code: &str, dj_name: &str) -> Result<(Session, Publisher), JoinError> {
        let mut sessions = self.sessions.lock().unwrap();

        // コードに一致するセッションを探す
        let session = sessions
            .values_mut()
            .find(|s| s.code == code)
            .ok_or(JoinError::InvalidCode)?;

        let publisher_id = format!("pub_{:03}", session.publishers.len() + 1);
        let token = format!("pt_{}", Uuid::new_v4().simple());

        let publisher = Publisher {
            id: publisher_id,
            dj_name: dj_name.to_string(),
            token,
        };

        session.publishers.push(publisher.clone());
        Ok((session.clone(), publisher))
    }
}

/// join 時のエラー
#[derive(Debug)]
pub enum JoinError {
    /// コードに一致するセッションが見つからない
    InvalidCode,
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
        viewer_token: session.viewer_token,
    };

    (StatusCode::CREATED, Json(response))
}

#[derive(Debug, Deserialize)]
pub struct JoinSessionRequest {
    pub code: String,
    pub dj_name: String,
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
    match store.join_by_code(&body.code, &body.dj_name) {
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
