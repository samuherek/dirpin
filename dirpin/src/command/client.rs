use clap::Parser;
use eyre::Result;

mod account;
mod add;
mod info;
mod key;
mod list;
mod search;
mod status;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    Info,
    Key,
    Doctor,
    Add(add::Cmd),
    List(list::Cmd),
    Search(search::Cmd),
    #[command(subcommand)]
    Account(account::Cmd),
    Sync,
    Status,
}

impl Cmd {
    #[tokio::main]
    pub async fn run(self) -> Result<()> {
        let settings = dirpin_client::settings::Settings::new()?;

        match self {
            Self::Info => info::run(&settings),
            Self::Status => status::run(&settings).await?,
            Self::Key => key::run(&settings)?,
            Self::Add(cmd) => cmd.run(&settings).await?,
            Self::Doctor => todo!("Show the debug info about the program and what the issue is"),
            Self::List(cmd) => cmd.run(),
            Self::Search(cmd) => cmd.run(),
            Self::Sync => todo!("Sync"),
            Self::Account(cmd) => cmd.run(),
        };

        Ok(())
    }
}
