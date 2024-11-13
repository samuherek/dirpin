use crate::api_client::AuthClient;
use crate::database::Database;
use crate::domain::{Entry, EntryDelete};
use crate::encryption::{decrypt, encrypt, load_key};
use crate::settings::Settings;
use crypto_secretbox::Key;
use dirpin_common::api::AddEntryRequest;
use eyre::Result;
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

/// Get the list of the updates (full data)
/// Get the list of the deletes (id, deleted_at)
/// Compare new updates with local db and buffer updates and conflicts
/// Compare new delets with local db and buffer updates and conflicts
async fn sync_download(
    settings: &Settings,
    db: &Database,
    session: &str,
    key: &Key,
    from: OffsetDateTime,
) -> Result<usize> {
    let res = AuthClient::new(&settings.server_address, session)?
        .sync(from)
        .await?;

    let local: HashMap<Uuid, Entry> = db
        .after(from)
        .await?
        .into_iter()
        .map(|x| (x.id.clone(), x))
        .collect();
    let remote: HashMap<Uuid, Entry> = res
        .updated
        .iter()
        .map(|x| serde_json::from_str(x).expect("failed deserialize"))
        .map(|x| decrypt(x, key).expect("failed to decrypt entry. check key!"))
        .map(|x| (x.id.clone(), x))
        .collect();

    let mut update_buf: Vec<Entry> = vec![];
    let mut conflict_buf: Vec<Entry> = vec![];
    let mut delete_buf: Vec<EntryDelete> = vec![];

    // Collect new versions into update buffer or conflict buffer.
    for (id, r) in remote {
        if let Some(l) = local.get(&id) {
            // If updated_at and version is higher, it's all good
            if r.updated_at > l.updated_at && r.version > l.version {
                update_buf.push(r);
            // if updated_at and version is equal, it's old and good,
            } else if r.updated_at == l.updated_at && r.version == l.version {
                continue;
            // if locals are higher, local will update later
            } else if r.updated_at < l.updated_at && r.version < l.version {
                continue;
            // otherwise we have a conflict;
            } else {
                conflict_buf.push(r);
            }
        } else {
            update_buf.push(r);
        }
    }

    let mut local: HashMap<Uuid, Entry> = db
        .list_deleted()
        .await?
        .into_iter()
        .map(|x| (x.id.clone(), x))
        .collect();
    let remote: HashMap<Uuid, EntryDelete> = res
        .deleted
        .into_iter()
        .filter_map(|x| x.try_into().ok())
        .map(|x: EntryDelete| (x.id.clone(), x))
        .collect();

    // Collect deleted entries into update buffer or conflict buffer.
    for (id, r) in remote {
        // If there is some updated in the local "from_last_sync" we need to check if there is any
        // possible conflicts.
        if let Some(l) = local.get_mut(&id) {
            // If version and updated_at of local are both lower or equal to remote, it's safe to delete.
            // Otherwise, other host made a change and delted before this sync happend. it's safe
            // to delete this too.
            if r.updated_at >= l.updated_at && r.version >= l.version {
                delete_buf.push(r);
            } else {
                l.deleted_at = Some(r.deleted_at);
                conflict_buf.push(l.clone());
            }
        // Otherwise the delete is newer than from last sync and it's safe to delete
        } else {
            delete_buf.push(r);
        }
    }

    db.save_bulk(&update_buf).await?;
    db.delete_bulk(&delete_buf).await?;

    if !conflict_buf.is_empty() {
        db.save_conflicts_bulk(&conflict_buf).await?;
        println!(
            "{} conflicts. Resolve in app before resyncing",
            conflict_buf.len()
        );
        std::process::exit(0);
    }

    Ok(update_buf.len())
}

/// The assumptoin for the logic of this function is that this function always runs after the
/// sync download function.
/// Meaning, we always download latest changes first and we ask the user to resolve any
/// conflicts before we hit this function. This means, that if the sync takes way too long and
/// there is a new update on the server, we will first check it before we upload.
/// Meaining, even if new values are in remote, we still download them first.
/// This is not buletproof, as there is a time in the
async fn sync_upload(
    settings: &Settings,
    db: &Database,
    session: &str,
    key: &Key,
    from: OffsetDateTime,
) -> Result<usize> {
    // TODO: Split this into pages so that we don't have massive payload.
    let items = db.after(from).await?;
    let deleted = db.deleted_after(from).await?;
    let mut buffer = vec![];

    for el in items.iter().chain(deleted.iter()) {
        let data = encrypt(el, key)?;
        let data = serde_json::to_string(&data)?;

        let p = AddEntryRequest {
            id: el.id.to_string(),
            data,
            version: el.version,
            updated_at: el.updated_at,
            deleted_at: el.deleted_at,
        };
        buffer.push(p);
    }

    AuthClient::new(&settings.server_address, session)?
        .post_entries(&buffer)
        .await?;

    Ok(buffer.len())
}

/// 1. Download recent changes from remote using last_sync_timestamp.
/// 2. Apply changes locally, tracking any unsynced local modifications or possible conflicts.
/// 3. After clean download, upload all new changes since last_sync_timestamp.
/// 4. Update last_sync_timestamp on successful sync.
///
/// This does not guarantee that all the changes from the remote will show up in the local as there
/// can be another update during this process that is missed. Due to this, we will rely on
/// periodical full local/remote comparison of versions to make sure we are all up to date
/// eventually.
pub async fn sync(settings: &Settings, db: &Database, force: bool) -> Result<()> {
    let session = settings.session();

    if session.is_none() {
        println!("Log in first!");
        return Ok(());
    }

    let from = Settings::last_sync()?;
    let key = load_key(settings)?;
    let session = session.unwrap();

    let from = if force {
        OffsetDateTime::UNIX_EPOCH
    } else {
        from
    };

    let down_count = sync_download(settings, db, &session, &key, from.clone()).await?;
    let up_count = sync_upload(settings, db, &session, &key, from).await?;

    println!("Sync done. {up_count} Uploaded / {down_count} Downloaded");
    Settings::save_last_sync()?;
    Ok(())
}
