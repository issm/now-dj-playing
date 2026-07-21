mod session;

use axum::{routing::get, routing::post, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;

use session::SessionStore;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() {
    let store = Arc::new(SessionStore::new());

    let api = Router::new()
        .route("/sessions/create", post(session::create_session))
        .route("/sessions/join", post(session::join_session))
        .with_state(store);

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api", api);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    eprintln!("ndp-server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
