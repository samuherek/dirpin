use crate::authentication::UserSession;
use crate::handlers::ServerError;
use crate::models::NewEntry;
use crate::router::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use dirpin_common::api::{AddEntryRequest, Deleted, SyncRequest, SyncResponse};
use tracing::error;

// TODO: make a propert error response types
pub async fn sync(
    session: UserSession,
    state: State<AppState>,
    params: Query<SyncRequest>,
) -> Result<Json<SyncResponse>, ServerError> {
    let user_id = session.user().id;
    // 1. Get the list of updates items from updated_at -> user specific
    // 2. convert the list to just the data
    //
    // 3. Get the list of new delets from deleted_at -> user specific
    // 4. convert the list to just the id and timestamp

    // TODO: get items for the user based on the last updated at
    // user_id
    // updated at after...
    // page size -> for now ignore
    // // TODO: page size
    let res = state
        .database
        .list_entries(user_id, params.last_sync_ts)
        .await
        .map_err(|err| {
            error!("Failed to list entries {err}");
            ServerError::DatabaseError("list entries")
        })?;

    let updated = res.into_iter().map(|x| x.data).collect::<Vec<_>>();

    let res = state
        .database
        .list_entries_deleted(user_id, params.last_sync_ts)
        .await
        .map_err(|err| {
            error!("Failed to list entries {err}");
            ServerError::DatabaseError("list entries")
        })?;

    let deleted = res
        .into_iter()
        .map(|x| Deleted {
            client_id: x.client_id,
            version: x.version,
            updated_at: x.updated_at,
            deleted_at: x.deleted_at.expect("failed to get deleted_at field"),
        })
        .collect::<Vec<_>>();

    Ok(Json(SyncResponse { updated, deleted }))
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
