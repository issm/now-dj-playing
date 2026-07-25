mod leave;
mod publish;
mod session;
mod stream;

use axum::{routing::delete, routing::get, routing::post, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use session::SessionStore;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("BUILD_VERSION_FULL"),
    })
}

#[tokio::main]
async fn main() {
    // tracing の初期化（RUST_LOG 環境変数で制御可能、デフォルト info + tower_http は debug）
    // JSON 形式で構造化ログを出力
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug")),
        )
        .init();

    let store = Arc::new(SessionStore::new());

    // CORS: 開発時は全オリジン許可。本番では Caddy が前段にいるため影響なし
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .route("/sessions/create", post(session::create_session))
        .route("/sessions/join", post(session::join_session))
        .route("/sessions/{session_id}", delete(session::destroy_session))
        .route("/sessions/{session_id}/publish", post(publish::publish))
        .route("/sessions/{session_id}/leave", post(leave::leave))
        .route("/sessions/{session_id}/stream", get(stream::stream))
        .with_state(store);

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("ndp-server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
