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
use tracing::debug;
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
) -> Result<(usize, usize)> {
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

    let update_count = update_buf.len();
    let mut delete_count = 0;

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
        if let Some(l) = local.get_mut(&id) {
            delete_count += 1;
            // If either version or updated_at are higher locally, it means there is some conflict.
            if r.updated_at < l.updated_at || r.version < l.version {
                l.deleted_at = Some(r.deleted_at);
                conflict_buf.push(l.clone());
                continue;
            }
        }

        delete_buf.push(r);
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

    Ok((update_count, delete_count))
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
    force: bool,
) -> Result<usize> {
    // TODO: Split this into pages so that we don't have massive payload.
    let mut items = db.after(from).await?;
    let update_count = items.len();
    let mut buffer = vec![];

    if !force {
        items.extend(db.deleted_after(from).await?);
    }

    for el in &items {
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

    Ok(update_count)
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

    let (download_count, delete_count) =
        sync_download(settings, db, &session, &key, from.clone()).await?;
    let upload_count = sync_upload(settings, db, &session, &key, from, force).await?;

    println!(
        "Sync done. {upload_count} Uploaded / {delete_count} Deleted / {download_count} Downloaded"
    );
    Settings::save_last_sync()?;
    Ok(())
}
