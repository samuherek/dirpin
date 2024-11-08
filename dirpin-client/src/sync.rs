use crate::api_client;
use crate::database::Database;
use crate::domain::Pin;
use crate::encryption::{decrypt, encrypt, load_key};
use crate::settings::Settings;
use crypto_secretbox::Key;
use dirpin_common::api::AddPinRequest;
use eyre::Result;
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

async fn sync_download(
    settings: &Settings,
    db: &Database,
    key: &Key,
    from: OffsetDateTime,
) -> Result<usize> {
    let res = api_client::sync(&settings.server_address, from).await?;

    let local: HashMap<Uuid, Pin> = db
        .after(from)
        .await?
        .into_iter()
        .map(|x| (x.id.clone(), x))
        .collect();
    let remote: HashMap<Uuid, Pin> = res
        .updated
        .iter()
        .map(|x| serde_json::from_str(x).expect("failed deserialize"))
        .map(|x| decrypt(x, key).expect("failed to decrypt pin. check key!"))
        .map(|x| (x.id.clone(), x))
        .collect();

    let mut conflict_buf: Vec<Pin> = vec![];
    let mut update_buf: Vec<Pin> = vec![];

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

        let p = AddPinRequest {
            id: el.id.to_string(),
            timestamp: el.updated_at,
            version: el.version,
            data,
        };
        buffer.push(p);
    }

    api_client::post_pins(&settings.server_address, &buffer).await?;

    Ok(buffer.len())
}

pub async fn sync(settings: &Settings, db: &Database, force: bool) -> Result<()> {
    // 1. Download recent changes from remote using last_sync_timestamp.
    // 2. Apply changes locally, tracking any unsynced local modifications.
    // 3. Upload remaining local changes, ensuring full consistency.
    // 4. Update last_sync_timestamp on successful sync.
    let from = Settings::last_sync()?;
    let key = load_key(settings)?;
    let from = if force {
        OffsetDateTime::UNIX_EPOCH
    } else {
        from
    };
    let down_count = sync_download(settings, db, &key, from.clone()).await?;
    let up_count = sync_upload(settings, db, &key, from).await?;

    println!("Sync done. {up_count} Uploaded / {down_count} Downloaded");
    Settings::save_last_sync()?;
    Ok(())
}
