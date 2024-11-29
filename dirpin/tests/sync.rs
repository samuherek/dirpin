mod helpers;
use dirpin_client::api_client::AuthClient;
use dirpin_client::domain::entry::Entry;
use dirpin_common::api::{AddEntryRequest, AddSyncRequest};
use fake::faker::internet::en::{FreeEmail, Password, Username};
use fake::faker::lorem::en::Word;
use fake::Fake;
use helpers::spawn_sync_app;
use time::OffsetDateTime;

#[tokio::test]
async fn sync() {
    let server = spawn_sync_app().await.unwrap();
    let server_address = server.address();

    let username: String = Username().fake();
    let password: String = Password(3..24).fake();
    let email: String = FreeEmail().fake();
    let host_id = format!(
        "{}@{}",
        Username().fake::<String>(),
        Word().fake::<String>()
    );

    let register_session = dirpin_client::api_client::register(
        &server_address,
        &username,
        &email,
        &password,
        &host_id,
    )
    .await
    .unwrap();

    let client = AuthClient::new(&server_address, &register_session.session).unwrap();
    let now = OffsetDateTime::now_utc();
    let host_id = helpers::build_host_id();
    let data1 = Word().fake::<String>();
    let data2 = Word().fake::<String>();

    let entry1 = Entry::new(data1.clone(), data1.clone(), None, host_id.clone());
    let entry1 = AddEntryRequest {
        id: entry1.id.to_string(),
        version: entry1.version.inner(),
        data: data1.clone(),
        kind: "entry".into(),
        updated_at: entry1.updated_at,
        deleted_at: entry1.deleted_at,
    };

    let entry2 = Entry::new(data2.clone(), data2.clone(), None, host_id.clone());
    let entry2 = AddEntryRequest {
        id: entry2.id.to_string(),
        version: entry2.version.inner(),
        data: data2.clone(),
        kind: "entry".into(),
        updated_at: entry2.updated_at,
        deleted_at: entry2.deleted_at,
    };

    let request = AddSyncRequest {
        items: vec![entry1, entry2],
        last_sync_ts: now,
    };

    client.post_entries(&request).await.unwrap();
    let response = client.sync(OffsetDateTime::UNIX_EPOCH).await.unwrap();

    assert_eq!(response.deleted.len(), 0);
    assert_eq!(response.updated.len(), 2);

    let res1 = response
        .updated
        .iter()
        .find(|x| &x.data == &data1)
        .and_then(|x| Some(x.data.clone()));
    let res2 = response
        .updated
        .iter()
        .find(|x| &x.data == &data2)
        .and_then(|x| Some(x.data.clone()));

    assert_eq!(Some(data1), res1);
    assert_eq!(Some(data2), res2);
}
