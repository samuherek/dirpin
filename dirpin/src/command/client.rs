use clap::Parser;
use eyre::Result;

mod account;
mod add;
mod info;
mod key;
mod list;
mod search;
mod status;
mod sync;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    Info,
    Key,
    Doctor,
    Add(add::Cmd),
    List(list::Cmd),
    Sync(sync::Cmd),
    Search(search::Cmd),
    #[command(subcommand)]
    Account(account::Cmd),
    Status,
}

impl Cmd {
    #[tokio::main]
    pub async fn run(self) -> Result<()> {
        let settings = dirpin_client::settings::Settings::new()?;
        let db = dirpin_client::database::Database::new(&settings.db_path).await?;

        match self {
            Self::Info => info::run(&settings),
            Self::Status => status::run(&settings).await?,
            Self::Key => key::run(&settings)?,
            Self::Add(cmd) => cmd.run(&settings, &db).await?,
            Self::List(cmd) => cmd.run(&settings, &db).await?,
            Self::Sync(cmd) => cmd.run(&settings, &db).await?,
            Self::Search(cmd) => cmd.run(&settings, &db).await?,
            Self::Doctor => todo!("Show the debug info about the program and what the issue is"),
            Self::Account(cmd) => cmd.run(),
        };

        Ok(())
    }
}
