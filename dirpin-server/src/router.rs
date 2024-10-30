use super::handlers;
use axum::http;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

async fn not_found() -> impl IntoResponse {
    (http::StatusCode::NOT_FOUND, "404 not found")
}

pub fn router() -> Router {
    let routes = Router::new()
        .route("/", get(handlers::index))
        .route("/sync", post(handlers::sync));

    routes.fallback(not_found).layer(TraceLayer::new_for_http())
}
