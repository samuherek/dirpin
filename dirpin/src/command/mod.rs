use clap::Subcommand;
use eyre::Result;

mod client;
mod server;

#[derive(Subcommand)]
pub enum DirpinCmd {
    #[command(flatten)]
    Client(client::Cmd),

    #[command(subcommand)]
    Server(server::Cmd),
}

impl DirpinCmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Server(cmd) => cmd.run(),
            Self::Client(cmd) => cmd.run(),
        }
    }
}
