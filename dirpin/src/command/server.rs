use clap::Parser;
use dirpin_server::settings::Settings;
use eyre::Result;
use std::net::SocketAddr;
use tracing_subscriber::{self, fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    /// Start the remote server
    Start {
        /// Host address
        #[clap(long)]
        host: Option<String>,
        /// Port to bind
        #[clap(long, short)]
        port: Option<u16>,
    },
}

impl Cmd {
    #[tokio::main]
    pub async fn run(self) -> Result<()> {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env())
            .init();

        tracing::trace!(command = ?self, "server command");

        match self {
            Self::Start { host, port } => {
                let settings = Settings::new()?;
                let host = host.as_ref().unwrap_or(&settings.host);
                let port = port.unwrap_or(settings.port);
                let address = SocketAddr::new(host.parse()?, port);
                dirpin_server::launch(&settings, address).await
            }
        }
    }
}
