use crate::api_client::AuthClient;
use crate::database::Database;
use crate::domain::conflict::{Conflict, HasId};
use crate::domain::entry::Entry;
use crate::domain::workspace::{Workspace, WorkspaceId};
use crate::encryption::{decrypt, encrypt, load_key, EncryptedItem};
use crate::settings::Settings;
use crypto_secretbox::Key;
use dirpin_common::api::{AddEntryRequest, RefDelete, RefItem};
use dirpin_common::domain::SyncVersion;
use eyre::{bail, Result};
use std::collections::HashMap;
use std::hash::Hash;
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

trait HasSyncProperties {
    type Timestamp: PartialOrd + Clone;
    type Version: PartialOrd;

    fn updated_at(&self) -> &Self::Timestamp;
    fn version(&self) -> &Self::Version;
    // fn deleted_at(&self) -> Option<&Self::Timestamp>;
    fn set_deleted_at(&mut self, deleted_at: Self::Timestamp);
}

impl HasSyncProperties for Workspace {
    type Timestamp = OffsetDateTime;
    type Version = SyncVersion;

    fn updated_at(&self) -> &Self::Timestamp {
        &self.updated_at
    }

    fn version(&self) -> &Self::Version {
        &self.version
    }

    // fn deleted_at(&self) -> Option<&Self::Timestamp> {
    //     self.deleted_at.as_ref()
    // }

    fn set_deleted_at(&mut self, deleted_at: Self::Timestamp) {
        self.deleted_at = Some(deleted_at);
    }
}

impl HasSyncProperties for Entry {
    type Timestamp = OffsetDateTime;
    type Version = SyncVersion;

    fn updated_at(&self) -> &Self::Timestamp {
        &self.updated_at
    }

    fn version(&self) -> &Self::Version {
        &self.version
    }

    // fn deleted_at(&self) -> Option<&Self::Timestamp> {
    //     self.deleted_at.as_ref()
    // }

    fn set_deleted_at(&mut self, deleted_at: Self::Timestamp) {
        self.deleted_at = Some(deleted_at);
    }
}

impl HasSyncProperties for RefDelete {
    type Timestamp = OffsetDateTime;
    type Version = SyncVersion;

    fn updated_at(&self) -> &Self::Timestamp {
        &self.updated_at
    }

    fn version(&self) -> &Self::Version {
        &self.version
    }

    // fn deleted_at(&self) -> Option<&Self::Timestamp> {
    //     Some(&self.deleted_at)
    // }

    fn set_deleted_at(&mut self, _deleted_at: Self::Timestamp) {
        unreachable!()
    }
}

impl HasId for Entry {
    fn id(&self) -> &Uuid {
        &self.id
    }
}

impl HasId for Workspace {
    fn id(&self) -> &Uuid {
        self.id.inner()
    }
}

fn parse_remote_updates(
    items: Vec<RefItem>,
    key: &Key,
) -> Result<(HashMap<WorkspaceId, Workspace>, HashMap<Uuid, Entry>)> {
    let mut workspaces: HashMap<WorkspaceId, Workspace> = HashMap::new();
    let mut entries: HashMap<Uuid, Entry> = HashMap::new();

    let decrypted = items.iter().map(|x| {
        let decoded = EncryptedItem::from_json_base64(&x.data).expect("failed deserialize");
        (&x.kind, decoded)
    });

    for (kind, data) in decrypted {
        match kind.as_str() {
            "entry" => {
                let entry: Entry = decrypt(data, key).expect("failed to decrypt entry. check key!");
                entries.insert(entry.id, entry);
            }
            "workspace" => {
                let workspace: Workspace =
                    decrypt(data, key).expect("failed to decrypt worksapce. check key!");
                workspaces.insert(workspace.id.clone(), workspace);
            }
            value => bail!("Failed to recoghnize {value} remote entry"),
        }
    }

    Ok((workspaces, entries))
}

fn parse_remote_delets(
    items: Vec<RefDelete>,
) -> Result<(
    HashMap<WorkspaceId, RefDelete>,
    HashMap<Uuid, RefDelete>,
    Vec<RefDelete>,
)> {
    let mut workspaces: HashMap<WorkspaceId, RefDelete> = HashMap::new();
    let mut entries: HashMap<Uuid, RefDelete> = HashMap::new();
    let mut unknown = Vec::new();

    for item in items {
        match item.kind.as_str() {
            "workspace" => {
                let id = WorkspaceId::from_str(&item.client_id)?;
                workspaces.insert(id, item);
            }
            "entry" => {
                let id = Uuid::parse_str(&item.client_id)?;
                entries.insert(id, item);
            }
            _ => {
                unknown.push(item);
            }
        }
    }

    Ok((workspaces, entries, unknown))
}

async fn get_local_updates(
    db: &Database,
    from: &OffsetDateTime,
) -> Result<(HashMap<WorkspaceId, Workspace>, HashMap<Uuid, Entry>)> {
    let workspaces: HashMap<WorkspaceId, Workspace> = db
        .after_workspaces(*from)
        .await?
        .into_iter()
        .map(|x| (x.id.clone(), x))
        .collect();

    let entries: HashMap<Uuid, Entry> = db
        .after(*from)
        .await?
        .into_iter()
        .map(|x| (x.id.clone(), x))
        .collect();

    Ok((workspaces, entries))
}

async fn get_local_delets(
    db: &Database,
    from: &OffsetDateTime,
) -> Result<(HashMap<WorkspaceId, Workspace>, HashMap<Uuid, Entry>)> {
    let workspaces: HashMap<WorkspaceId, Workspace> = db
        .deleted_after_workspaces(*from)
        .await?
        .into_iter()
        .map(|x| (x.id.clone(), x))
        .collect();

    let entries: HashMap<Uuid, Entry> = db
        .deleted_after(*from)
        .await?
        .into_iter()
        .map(|x| (x.id.clone(), x))
        .collect();

    Ok((workspaces, entries))
}

fn collect_diff_updates<H, T>(
    remote: &HashMap<H, T>,
    local: &HashMap<H, T>,
    changes: &mut Vec<T>,
    conflicts: &mut Vec<Conflict>,
) -> Result<()>
where
    H: Hash + Eq,
    T: HasSyncProperties + Clone + HasId + serde::Serialize,
{
    for (id, r) in remote {
        if let Some(l) = local.get(&id) {
            let r_time = r.updated_at();
            let l_time = l.updated_at();
            let r_version = r.version();
            let l_version = l.version();

            // If updated_at and version is higher, it's all good
            if r_time > l_time && r_version > l_version {
                changes.push(r.clone());
            // if timed_at and version is equal, it's old and good,
            } else if r_time == l_time && r_version == l_version {
                continue;
            // if locals are higher, local will update later
            } else if r_time < l_time && r_version < l_version {
                continue;
            // otherwise we have a conflict;
            } else {
                let conflict = Conflict::from_serializable(r)?;
                conflicts.push(conflict);
            }
        } else {
            changes.push(r.clone());
        }
    }

    Ok(())
}

fn collect_diff_delets<H, T>(
    remote: &HashMap<H, RefDelete>,
    local: &HashMap<H, T>,
    changes: &mut Vec<RefDelete>,
    conflicts: &mut Vec<Conflict>,
) -> Result<()>
where
    H: Hash + Eq,
    T: HasSyncProperties<Timestamp = OffsetDateTime, Version = SyncVersion>
        + Clone
        + HasId
        + serde::Serialize,
    RefDelete: HasSyncProperties,
    OffsetDateTime: PartialOrd<<T as HasSyncProperties>::Timestamp>,
{
    for (id, r) in remote {
        if let Some(l) = local.get(&id) {
            // If either version or updated_at are higher locally, it means there is some conflict.
            if r.updated_at() < l.updated_at() || r.version() < l.version() {
                let mut item = l.clone();
                item.set_deleted_at(r.deleted_at.clone());
                let conflict = Conflict::from_serializable(&item)?;
                conflicts.push(conflict);
                continue;
            }
        }

        changes.push(r.clone());
    }

    Ok(())
}

struct DownloadStatus {
    workspace_delets: usize,
    workspace_updates: usize,
    entry_delets: usize,
    entry_updates: usize,
}

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
) -> Result<DownloadStatus> {
    let res = AuthClient::new(&settings.server_address, session)?
        .sync(from)
        .await?;

    let (remote_workspace_ups, remote_entry_ups) = parse_remote_updates(res.updated, key)?;
    let (remote_workspace_dels, remote_entry_dels, unknown_dels) =
        parse_remote_delets(res.deleted)?;
    let (local_workspace_ups, local_entry_ups) = get_local_updates(db, &from).await?;
    let (local_workspace_dels, local_entry_dels) = get_local_delets(db, &from).await?;

    if !unknown_dels.is_empty() {
        bail!(
            "Found {} unknown deletion kinds in server resopnse",
            unknown_dels.len()
        );
    }

    let mut update_workspaces: Vec<Workspace> = vec![];
    let mut update_entries: Vec<Entry> = vec![];
    let mut delete_workspaces: Vec<RefDelete> = vec![];
    let mut delete_entries: Vec<RefDelete> = vec![];
    let mut conflicts: Vec<Conflict> = vec![];

    collect_diff_updates(
        &remote_workspace_ups,
        &local_workspace_ups,
        &mut update_workspaces,
        &mut conflicts,
    )?;
    collect_diff_updates(
        &remote_entry_ups,
        &local_entry_ups,
        &mut update_entries,
        &mut conflicts,
    )?;

    collect_diff_delets(
        &remote_workspace_dels,
        &local_workspace_dels,
        &mut delete_workspaces,
        &mut conflicts,
    )?;
    collect_diff_delets(
        &remote_entry_dels,
        &local_entry_dels,
        &mut delete_entries,
        &mut conflicts,
    )?;

    // Update first. Workspaces must go first.
    db.save_workspace_bulk(&update_workspaces).await?;
    db.save_bulk(&update_entries).await?;

    // Delete second
    db.delete_workspace_ref_bulk(&delete_workspaces).await?;
    db.delete_ref_bulk(&delete_entries).await?;

    // Check for conflicts
    if !conflicts.is_empty() {
        db.save_conflicts_bulk(&conflicts).await?;
        println!(
            "{} conflicts. Resolve in app before resyncing",
            conflicts.len()
        );
        std::process::exit(0);
    }

    Ok(DownloadStatus {
        workspace_updates: update_workspaces.len(),
        workspace_delets: delete_workspaces.len(),
        entry_updates: update_entries.len(),
        entry_delets: delete_entries.len(),
    })
}

struct UploadStatus {
    entries: usize,
    workspaces: usize,
}

/// The assumptoin for the logic of this function is that this function always runs after the
/// sync download function.
/// Meaning, we always download latest changes first and we ask the user to resolve any
/// conflicts before we hit this function. This means, that if the sync takes way too long and
/// there is a new update on the server, we will first check it before we upload.
/// Meaining, even if new values are in remote, we still download them first.
/// This is not buletproof, as there is a time in-between that can create new values from a
/// different host. However, we'll use a periodic doctor to spot this.
async fn sync_upload(
    settings: &Settings,
    db: &Database,
    session: &str,
    key: &Key,
    from: OffsetDateTime,
) -> Result<UploadStatus> {
    // TODO: Split this into pages so that we don't have massive payload.
    let mut workspaces = db.after_workspaces(from).await?;
    workspaces.extend(db.deleted_after_workspaces(from).await?);

    let mut entries = db.after(from).await?;
    entries.extend(db.deleted_after(from).await?);

    let mut buffer = vec![];

    for entry in &entries {
        buffer.push(AddEntryRequest {
            id: entry.id.to_string(),
            data: encrypt(entry, key)?.to_json_base64()?,
            kind: "entry".into(),
            version: entry.version.inner(),
            updated_at: entry.updated_at,
            deleted_at: entry.deleted_at,
        });
    }

    for ws in &workspaces {
        buffer.push(AddEntryRequest {
            id: ws.id.to_string(),
            data: encrypt(ws, key)?.to_json_base64()?,
            kind: "workspace".into(),
            version: ws.version.inner(),
            updated_at: ws.updated_at,
            deleted_at: ws.deleted_at,
        });
    }

    AuthClient::new(&settings.server_address, session)?
        .post_entries(&buffer)
        .await?;

    Ok(UploadStatus {
        entries: entries.len(),
        workspaces: workspaces.len(),
    })
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

    let down_status = sync_download(settings, db, &session, &key, from.clone()).await?;
    let up_status = sync_upload(settings, db, &session, &key, from).await?;

    println!(
        "Workspaces: {} Uploaded / {} Deleted / {} Downloaded",
        up_status.workspaces, down_status.workspace_delets, down_status.workspace_updates
    );
    println!(
        "Entries: {} Uploaded / {} Deleted / {} Downloaded",
        up_status.entries, down_status.entry_delets, down_status.entry_updates
    );
    Settings::save_last_sync()?;

    Ok(())
}
