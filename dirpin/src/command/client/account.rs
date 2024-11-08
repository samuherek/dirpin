use clap::Parser;
use dirpin_client::settings::Settings;
use eyre::Result;

mod login;
mod register;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    /// Login to the remote account for syncing
    Login(login::Cmd),
    /// Register a remote account for syncing
    Register(register::Cmd),
    Logout,
    Delete,
    /// Verification step of the new account on the server
    Verify,
}

impl Cmd {
    pub(crate) async fn run(self, settings: &Settings) -> Result<()> {
        match self {
            Self::Register(cmd) => cmd.run(settings).await?,
            Self::Login(cmd) => cmd.run(settings).await?,
            Self::Logout => todo!("Logout"),
            Self::Delete => todo!("Delete"),
            Self::Verify => todo!("Verify"),
        }

        Ok(())
    }
}
