use super::handlers;
use crate::database::Database;
use axum::http;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
}

async fn not_found() -> impl IntoResponse {
    (http::StatusCode::NOT_FOUND, "404 not found")
}

pub fn router(database: Database) -> Router {
    let routes = Router::new()
        .route("/", get(handlers::index))
        .route("/sync", get(handlers::sync))
        .route("/pins", post(handlers::add))
        .route("/register", post(handlers::register));

    routes
        .fallback(not_found)
        .with_state(AppState { database })
        .layer(TraceLayer::new_for_http())
}
