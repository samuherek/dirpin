use super::handlers;
use axum::routing::get;
use axum::Router;

pub fn router() -> Router {
    Router::new().route("/", get(handlers::index))
}
