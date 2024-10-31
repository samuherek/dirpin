use crate::api_client;
use crate::database::Database;
use crate::settings::Settings;
use dirpin_common::api::AddPinRequest;
use eyre::Result;
use time::OffsetDateTime;

async fn sync_download(settings: &Settings, _db: &Database, from: OffsetDateTime) -> Result<()> {
    let res = api_client::handle_sync(&settings.server_address, from).await?;

    println!("sync down res: {res:?}");
    if res.updated.is_empty() && res.deleted.is_empty() {
        println!("All up to date");
    } else {
        todo!("Have to implement the download sync");
    }

    Ok(())
}

async fn sync_upload(
    settings: &Settings,
    db: &Database,
    force: bool,
    from: OffsetDateTime,
) -> Result<()> {
    let from = if force {
        OffsetDateTime::UNIX_EPOCH
    } else {
        from
    };
    let items = db.after(from, 10).await?;
    let mut buffer = vec![];

    for el in &items {
        // TODO: Encryt it
        let data = serde_json::to_string(el)?;

        let p = AddPinRequest {
            id: el.id.to_string(),
            timestamp: el.updated_at,
            version: el.version,
            data,
        };
        buffer.push(p);
    }

    api_client::handle_post_pins(&settings.server_address, &buffer).await?;

    println!("items: {items:?}");
    Ok(())
}

pub async fn sync(settings: &Settings, db: &Database, force: bool) -> Result<()> {
    // 1. Download recent changes from remote using last_sync_timestamp.
    // 2. Apply changes locally, tracking any unsynced local modifications.
    // 3. Upload remaining local changes, ensuring full consistency.
    // 4. Update last_sync_timestamp on successful sync.
    let from = Settings::last_sync()?;
    sync_download(settings, db, from.clone()).await?;
    sync_upload(settings, db, force, from).await?;

    println!("Done sync");
    Settings::save_last_sync()?;
    Ok(())
}
