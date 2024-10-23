use clap::Parser;
use eyre::Result;

mod account;
mod info;
mod list;
mod search;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    Info,
    Key,
    Doctor,
    List(list::Cmd),
    Search(search::Cmd),
    #[command(subcommand)]
    Account(account::Cmd),
    Sync,
}

impl Cmd {
    #[tokio::main]
    pub async fn run(self) -> Result<()> {
        let settings = dirpin_client::settings::Settings::new()?;

        match self {
            Self::Info => info::run(&settings),
            Self::Doctor => todo!("Show the debug info about the program and what the issue is"),
            Self::List(cmd) => cmd.run(),
            Self::Search(cmd) => cmd.run(),
            Self::Sync => todo!("Sync"),
            Self::Account(cmd) => cmd.run(),
            Self::Key => todo!("generate and show the key for this account"),
        };

        Ok(())
    }
}
