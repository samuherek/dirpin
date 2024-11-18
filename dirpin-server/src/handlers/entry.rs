use crate::authentication::UserSession;
use crate::error::ServerError;
use crate::models::{Entry, NewEntry};
use crate::router::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use dirpin_common::api::{AddSyncRequest, RefDelete, RefItem, SyncRequest, SyncResponse};
use std::collections::HashMap;
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

    // TODO: page size
    let res = state
        .database
        .list_entries(user_id, params.last_sync_ts)
        .await
        .map_err(|err| {
            error!("Failed to list entries {err}");
            ServerError::DatabaseError("list entries")
        })?;

    let updated = res
        .into_iter()
        .map(|x| RefItem {
            data: x.data,
            kind: x.kind,
        })
        .collect::<Vec<_>>();

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
        .map(|x| RefDelete {
            client_id: x.client_id,
            version: x.version.into(),
            updated_at: x.updated_at,
            deleted_at: x.deleted_at.expect("failed to get deleted_at field"),
            kind: x.kind,
        })
        .collect::<Vec<_>>();

    Ok(Json(SyncResponse { updated, deleted }))
}

pub async fn add(
    session: UserSession,
    state: State<AppState>,
    Json(req): Json<AddSyncRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let user = session.user();

    let mut client_updates: HashMap<String, NewEntry> = HashMap::new();
    let mut client_deletes: HashMap<String, NewEntry> = HashMap::new();

    for item in req.items {
        let new_entry = NewEntry {
            client_id: item.id,
            user_id: user.id.into(),
            version: item.version,
            data: item.data,
            kind: item.kind,
            updated_at: item.updated_at,
            deleted_at: item.deleted_at,
        };

        match new_entry.deleted_at {
            Some(_) => client_deletes.insert(new_entry.client_id.clone(), new_entry),
            None => client_updates.insert(new_entry.client_id.clone(), new_entry),
        };
    }

    let server_entries: HashMap<String, Entry> = state
        .database
        .list_changed_from(user.id, req.last_sync_ts)
        .await
        .map_err(|err| {
            error!("Failed to add entries {err}");
            ServerError::DatabaseError("add entries")
        })?
        .into_iter()
        .map(|x| (x.client_id.clone(), x))
        .collect();

    let mut update_buff = vec![];

    // from timestamp
    // - if we have an updated item
    //  - check if version and timestmap are higher
    //  - otherwise report
    //  - if there is no such an item at all, just add it to the db.
    //  - if the item has already been deleted, report
    //
    // - if we have deleted item
    //  - if timestamp is newer than deleted, we report
    //  - if deleted timestamp is newer, we skip.
    //  - otherwise we really don't care and just delete.
    for (id, c) in client_updates {
        match (server_entries.get(&id), c.deleted_at) {
            (Some(s), None) => {
                if s.deleted_at.is_some() {
                    return Err(ServerError::Conflict("Updating a deleted item."));
                } else if c.updated_at >= s.updated_at && c.version >= s.version {
                    update_buff.push(c);
                } else {
                    return Err(ServerError::Conflict("Updating an entry."));
                }
            }
            (Some(s), Some(del_at)) => {
                if s.updated_at > del_at {
                    return Err(ServerError::Conflict(
                        "Trying to delete newer entry with older delete timestamp.",
                    ));
                } else {
                    update_buff.push(c);
                }
            }
            (None, _) => {
                update_buff.push(c);
            }
        };
    }

    state
        .database
        .save_entries(&update_buff)
        .await
        .map_err(|err| {
            error!("Failed to add entries {err}");
            ServerError::DatabaseError("add entries")
        })?;

    Ok(StatusCode::OK)
}
