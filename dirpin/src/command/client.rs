use clap::Parser;
use eyre::Result;

mod account;
mod list;
mod search;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    Doctor,
    Info,
    List(list::Cmd),
    Search(search::Cmd),
    #[command(subcommand)]
    Account(account::Cmd),
    Sync,
    Key,
}

impl Cmd {
    #[tokio::main]
    pub async fn run(self) -> Result<()> {
        let res = dirpin_client::settings::Settings::new();
        println!("settings: {res:?}");

        match self {
            Self::Doctor => todo!("Show the debug info about the program and what the issue is"),
            Self::Info => todo!("Show the config file paths"),
            Self::List(cmd) => cmd.run(),
            Self::Search(cmd) => cmd.run(),
            Self::Sync => todo!("Sync"),
            Self::Account(cmd) => cmd.run(),
            Self::Key => todo!("generate and show the key for this account"),
        };

        Ok(())
    }
}
