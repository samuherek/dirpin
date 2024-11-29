use axum::serve;
use dirpin_client::database::Database as ClientDatabase;
use dirpin_client::domain::entry::{Entry};
use dirpin_client::domain::host::HostId;
use dirpin_client::settings::Settings as ClientSettings;
use dirpin_server::database::Database as ServerDatabase;
use dirpin_server::make_router;
use dirpin_server::settings::Settings as ServerSettings;
use eyre::{eyre, Result};
use tokio::net::TcpListener;

struct TestClient {
    pub settings: ClientSettings,
    pub database: ClientDatabase,
}

impl TestClient {
    pub async fn build() -> Result<Self> {
        let temp_dir = tempfile::TempDir::new()?;
        let dir_path = temp_dir.path();
        let key_path = dir_path.join("key");
        let session_path = dir_path.join("session");

        let settings: ClientSettings = ClientSettings::build_default()?
            .set_default("db_path", "sqlite::memory:")?
            .set_default("key_path", key_path.to_str())?
            .set_default("session_path", session_path.to_str())?
            .set_default("server_address", "")?
            .build()?
            .try_deserialize()
            .map_err(|e| eyre!("Failed to deseriazlie {e}"))?;

        let database = ClientDatabase::new(&settings.db_path).await?;
        sqlx::migrate!("../dirpin-client/migrations")
            .run(&database.pool)
            .await?;

        Ok(Self { settings, database })
    }

    pub fn set_server_address(&mut self, address: &str) {
        self.settings.server_address = address.into();
    }
}

struct TestServer {
    pub settings: ServerSettings,
    pub database: ServerDatabase,
}

impl TestServer {
    pub async fn build(host: &str, port: u16) -> Result<Self> {
        let settings = ServerSettings::build_default()?;
        let settings: ServerSettings = settings
            .set_default("db_path", "sqlite::memory:")?
            .set_default("port", port)?
            .set_default("host", host)?
            .build()?
            .try_deserialize()
            .map_err(|e| eyre!("Failed to deseriazlie {e}"))?;

        let database = ServerDatabase::new(&settings.db_path).await?;
        sqlx::migrate!("../dirpin-server/migrations")
            .run(&database.pool)
            .await?;

        Ok(TestServer { settings, database })
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.settings.host, self.settings.port)
    }
}

struct TestSyncApp {
    pub client: TestClient,
    pub server: TestServer,
}

async fn spawn_sync_app() -> Result<TestSyncApp> {
    let host = "127.0.0.1";
    let mut port = 0;
    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    port = listener.local_addr().unwrap().port();

    let server = TestServer::build(&host, port).await?;
    let mut client = TestClient::build().await?;
    client.set_server_address(&server.address());
    let app = TestSyncApp { client, server };

    let r = make_router(&app.server.settings, app.server.database.clone()).await;
    let _ = tokio::spawn(async move { serve(listener, r.into_make_service()).await.unwrap() });
    Ok(app)
}

#[tokio::test]
async fn connecting_to_server_sync() -> Result<()> {
    let app = spawn_sync_app().await?;

    let entry = Entry::new("test".into(), "/hellow".into(), None, HostId::get_host_id());
    app.client.database.save(&entry).await?;

    let res = sqlx::query("select * from entries")
        .fetch_all(&app.client.database.pool)
        .await?;

    assert_eq!(res.len(), 1);
    assert!(true);

    Ok(())
}
