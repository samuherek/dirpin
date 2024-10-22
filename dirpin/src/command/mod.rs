use clap::Subcommand;
use eyre::Result;

mod client;
mod server;

#[derive(Subcommand)]
pub enum DirpinCmd {
    #[command(subcommand)]
    Server(server::Cmd),
    #[command(subcommand)]
    Client(client::Cmd),
}

impl DirpinCmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Server(cmd) => cmd.run(),
            Self::Client(cmd) => cmd.run(),
        }
    }
}
