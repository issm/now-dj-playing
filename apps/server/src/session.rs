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
        };

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(id, session.clone());
        session
    }
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
