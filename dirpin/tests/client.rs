mod helpers;
use dirpin_client::domain::entry::Entry;
use dirpin_client::domain::host::HostId;
use helpers::{spawn_sync_app, TestClient};

#[tokio::test]
async fn connecting() {
    let app = spawn_sync_app().await.unwrap();
    let client = TestClient::build().await.unwrap();

    let entry = Entry::new("test".into(), "/hellow".into(), None, HostId::get_host_id());
    client.database.save(&entry).await.unwrap();

    let res = sqlx::query("select * from entries")
        .fetch_all(&client.database.pool)
        .await
        .unwrap();

    assert_eq!(res.len(), 1);
    assert!(true);
}

#[tokio::test]
async fn info_command_works() {
    let client = TestClient::build().await.unwrap();
}
