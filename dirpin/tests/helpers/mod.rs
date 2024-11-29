use axum::serve;
use dirpin_client::database::Database as ClientDatabase;
use dirpin_client::settings::Settings as ClientSettings;
use dirpin_server::database::Database as ServerDatabase;
use dirpin_server::make_router;
use dirpin_server::settings::Settings as ServerSettings;
use eyre::{eyre, Result};
use tokio::net::TcpListener;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct TestClient {
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

pub struct TestServer {
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
        format!("http://{}:{}", self.settings.host, self.settings.port)
    }
}

pub struct TestSyncApp {
    pub client: TestClient,
    pub server: TestServer,
}

pub async fn spawn_sync_app() -> Result<TestServer> {
    let host = "127.0.0.1";
    let mut port = 0;
    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    port = listener.local_addr().unwrap().port();

    let server = TestServer::build(&host, port).await?;

    let r = make_router(&server.settings, server.database.clone()).await;
    let _ = tokio::spawn(async move { serve(listener, r.into_make_service()).await.unwrap() });
    Ok(server)
}
