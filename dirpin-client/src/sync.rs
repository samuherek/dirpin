use crate::api_client::AuthClient;
use crate::database::Database;
use crate::domain::Entry;
use crate::encryption::{decrypt, encrypt, load_key};
use crate::settings::Settings;
use crypto_secretbox::Key;
use dirpin_common::api::AddEntryRequest;
use eyre::Result;
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

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
        .map(|x| decrypt(x, key).expect("failed to decrypt pin. check key!"))
        .map(|x| (x.id.clone(), x))
        .collect();

    let mut conflict_buf: Vec<Entry> = vec![];
    let mut update_buf: Vec<Entry> = vec![];

    for (id, r) in remote {
        if let Some(l) = local.get(&id) {
            // If updated_at and version is higher, it's all good
            // if updated_at and version is equal, it's old and good,
            // if locals are higher, local will update later
            // otherwise we have a conflict;
            if r.updated_at > l.updated_at && r.version > l.version {
                update_buf.push(r);
            } else if r.updated_at == l.updated_at && r.version == l.version {
                continue;
            } else if r.updated_at < l.updated_at && r.version < l.version {
                continue;
            } else {
                conflict_buf.push(r);
            }
        } else {
            update_buf.push(r);
        }
    }

    db.save_bulk(&update_buf).await?;

    if !conflict_buf.is_empty() {
        println!("conflicts: {conflict_buf:?}");
    }

    Ok(update_buf.len())
}

async fn sync_upload(
    settings: &Settings,
    db: &Database,
    session: &str,
    key: &Key,
    from: OffsetDateTime,
) -> Result<usize> {
    // TODO: Split this into pages so that we don't have massive payload.
    let items = db.after(from).await?;
    let mut buffer = vec![];

    for el in &items {
        // TODO: Encryt it
        let data = encrypt(el, key)?;
        let data = serde_json::to_string(&data)?;

        let p = AddEntryRequest {
            id: el.id.to_string(),
            version: el.version,
            data,
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

pub async fn sync(settings: &Settings, db: &Database, force: bool) -> Result<()> {
    // 1. Download recent changes from remote using last_sync_timestamp.
    // 2. Apply changes locally, tracking any unsynced local modifications.
    // 3. Upload remaining local changes, ensuring full consistency.
    // 4. Update last_sync_timestamp on successful sync.
    let session = settings.session();

    if session.is_none() {
        println!("Log in to use syncing!");
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
