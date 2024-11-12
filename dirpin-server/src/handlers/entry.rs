use crate::authentication::UserSession;
use crate::handlers::ServerError;
use crate::models::NewEntry;
use crate::router::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use dirpin_common::api::{AddEntryRequest, SyncRequest, SyncResponse};
use tracing::error;

// TODO: make a propert error response types
pub async fn sync(
    _session: UserSession,
    state: State<AppState>,
    _params: Query<SyncRequest>,
) -> Result<Json<SyncResponse>, ServerError> {
    // user_id
    // updated at after...
    //
    // page size -> for now ignore
    let res = state.database.list_entries().await.map_err(|err| {
        error!("Failed to list entries {err}");
        ServerError::DatabaseError("list entries")
    })?;

    Ok(Json(SyncResponse {
        updated: res.into_iter().map(|x| x.data).collect::<Vec<_>>(),
        deleted: vec![],
    }))
}

pub async fn add(
    _session: UserSession,
    state: State<AppState>,
    Json(req): Json<Vec<AddEntryRequest>>,
) -> Result<impl IntoResponse, ServerError> {
    let entries = req
        .into_iter()
        .map(|x| NewEntry {
            client_id: x.id,
            user_id: 1,
            version: x.version,
            data: x.data,
            updated_at: x.updated_at,
            deleted_at: x.deleted_at,
        })
        .collect::<Vec<_>>();

    state.database.add_entries(&entries).await.map_err(|err| {
        error!("Failed to add entries {err}");
        ServerError::DatabaseError("add entries")
    })?;

    Ok(StatusCode::OK)
}
