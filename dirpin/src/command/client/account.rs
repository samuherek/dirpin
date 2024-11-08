use clap::Parser;
use dirpin_client::settings::Settings;
use eyre::Result;

mod register;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct LoginCmd {
    u: String,
    p: String,
    k: String,
}

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    Login(LoginCmd),
    Register(register::Cmd),
    Logout,
    Delete,
}

impl Cmd {
    pub(crate) async fn run(self, settings: &Settings) -> Result<()> {
        println!("Account");
        match self {
            Self::Register(cmd) => cmd.run(settings).await?,
            Self::Login(_) => todo!("Login"),
            Self::Logout => todo!("Logout"),
            Self::Delete => todo!("Delete"),
        }

        Ok(())
    }
}
