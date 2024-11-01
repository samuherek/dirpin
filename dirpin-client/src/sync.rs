use crate::api_client;
use crate::database::Database;
use crate::encryption::{decrypt, encrypt, load_key, EncryptedPin};
use crate::settings::Settings;
use crypto_secretbox::Key;
use dirpin_common::api::AddPinRequest;
use eyre::Result;
use time::OffsetDateTime;

async fn sync_download(
    settings: &Settings,
    db: &Database,
    key: &Key,
    _from: OffsetDateTime,
) -> Result<()> {
    let from = OffsetDateTime::UNIX_EPOCH;
    let res = api_client::sync(&settings.server_address, from).await?;

    if res.updated.is_empty() && res.deleted.is_empty() {
        println!("All up to date");
    } else {
        let data: Vec<_> = res
            .updated
            .iter()
            .map(|x| serde_json::from_str(x).expect("failed deserialize"))
            .map(|x| decrypt(x, key).expect("failed to decrypt pin. check key!"))
            .collect();
        db.save_bulk(&data).await?;
    }

    Ok(())
}

async fn sync_upload(
    settings: &Settings,
    db: &Database,
    key: &Key,
    force: bool,
    from: OffsetDateTime,
) -> Result<()> {
    let from = if force {
        OffsetDateTime::UNIX_EPOCH
    } else {
        from
    };
    // TODO: Split this into pages so that we don't have massive payload.
    let items = db.after(from, 1000).await?;
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

    Ok(())
}

pub async fn sync(settings: &Settings, db: &Database, force: bool) -> Result<()> {
    // 1. Download recent changes from remote using last_sync_timestamp.
    // 2. Apply changes locally, tracking any unsynced local modifications.
    // 3. Upload remaining local changes, ensuring full consistency.
    // 4. Update last_sync_timestamp on successful sync.
    let from = Settings::last_sync()?;
    let key = load_key(settings)?;
    sync_download(settings, db, &key, from.clone()).await?;
    sync_upload(settings, db, &key, force, from).await?;

    println!("Done sync");
    Settings::save_last_sync()?;
    Ok(())
}
