mod helpers;
use dirpin_client::api_client::AuthClient;
use fake::faker::internet::en::{FreeEmail, Password, Username};
use fake::Fake;
use helpers::spawn_sync_app;

#[tokio::test]
async fn registration() {
    let server = spawn_sync_app().await.unwrap();
    let server_address = server.address();

    let username: String = Username().fake();
    let password: String = Password(3..24).fake();
    let email: String = FreeEmail().fake();
    let host_id = helpers::build_host_id().to_string();

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
    let status = client.status().await.unwrap();

    assert_eq!(status.username, username);
    assert_eq!(status.version, helpers::VERSION);

    let login_session =
        dirpin_client::api_client::login(&server_address, &username, &password, &host_id)
            .await
            .unwrap();

    let client = AuthClient::new(&server_address, &login_session.session).unwrap();
    let status = client.status().await.unwrap();

    assert_eq!(status.username, username);
    assert_eq!(status.version, helpers::VERSION);
}
