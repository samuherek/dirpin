mod helpers;
use dirpin_common::api::HealthCheckResponse;
use helpers::{spawn_sync_app, VERSION};

#[tokio::test]
async fn health_check() {
    let server = spawn_sync_app().await.unwrap();
    let response = dirpin_client::api_client::health_check(&server.address())
        .await
        .unwrap();

    assert_eq!(
        HealthCheckResponse {
            status: "Ok".into(),
            version: VERSION.into()
        },
        response
    );
}
